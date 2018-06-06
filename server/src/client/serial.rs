use client::context::*;

use std::io::{ Write, Read };
use std::fs::File;

//
pub struct SerialClient {
    context:        Context,
    source_file:    File,
    source_path:    String,
}

//
impl SerialClient {

    // create new serial client
    pub fn new(context: Context, source_path: String) -> Self {

        //
        let mut source_file = match File::create(&source_path) {
            Ok(file)    => file,
            Err(_)      => panic!("[ client ] [ {} ] unable to open serial source file", context.name)
        };

        //
        Self {
            context:        context,
            source_file:    source_file,
            source_path:    source_path,
        }
    }
}

//
impl Client for SerialClient {

    //
    fn start(&mut self) {
        let name = self.context.name.clone();
        let source_path = self.source_path.clone();
        let kernel_path = match &self.context.binary {
            Some(binary)    => binary.clone(),
            None            => panic!("[ client ] [ {} ] no binary file specified", name)
        };
        let locked = self.context.locked.clone();
        thread::spawn(|| { start(name, source_path, kernel_path, locked) });
    }

    //
    #[cfg(feature = "controller")]
    fn event(&mut self, data: u16) {
        if self.context.locked.load(Ordering::Relaxed) == false {
            self.source_file.write(&[(data >> 8) as u8, data as u8]).unwrap();
        }
    }

    //
    #[cfg(feature = "controller")]
    fn index(&self) -> usize {
        self.context.index
    }
}

//
fn read_request(source_path: &str) -> Request {

    // open serial source file
    let mut source_file = match File::open(source_path) {
        Ok(file)    => file,
        Err(_)      => return Request::Terminate(String::from("unable to open serial source file"))
    };

    //
    let mut buffer = [0];

    // read incoming characters and break for transmition to start
    loop {
        source_file.read_exact(&mut buffer).unwrap();
        match buffer[0] {
            b'?'    => {
                source_file.read_exact(&mut buffer).unwrap();
                return match buffer[0] {

                    //
                    b'k'    => Request::TransmitKernel,

                    //
                    b'f'    => {
                        let mut path = String::new();
                        loop {

                            //
                            if path.len() >= 128 {
                                return Request::Terminate(String::from("exeeded path length"))
                            }

                            //
                            source_file.read_exact(&mut buffer).unwrap();
                            match buffer[0] {
                                b'?'    => break,
                                byte    => path.push(byte as char),
                            }
                        }
                        Request::TransmitFile(path)
                    },

                    // invalid request
                    request => return Request::Terminate(format!("invalid request '{}'", request as char))
                }
            },
            byte    => print!("{}", byte as char),
        }
    }
}

//
fn transmit_file(name: &str, source_path: &str, path: &str) {
    use std::time::Instant;

    // open serial source file and kernel binary file
    if let Ok(binary) = File::open(path) {
        let time = Instant::now();

        // send initial transmition character and send the size
        let size = file_size(path);
        println!("[ client ] [ {} ] transmitting 0x{:x} bytes", name, size);

        //
        let mut source_file = match File::create(source_path) {
            Ok(file)    => file,
            Err(_)      => panic!("[ client ] [ {} ] unable to open serial source file", name)
        };

        //
        let buffer: [u8; 5] = [b'!', (size >> 24) as u8, (size >> 16) as u8, (size >> 8) as u8, size as u8];
        source_file.write_all(&buffer).unwrap();

        // iterate over every byte in the binary and send it
        for byte in binary.bytes() {
            source_file.write(&[byte.unwrap()]).unwrap();
        }

        println!("[ client ] [ {} ] transmitted in {} second/s", name, time.elapsed().as_secs());
    } else {
        println!("[ client ] [ {} ] unable to open binary file '{}'", name, path);
    }
}

//
fn start(name: String, source_path: String, kernel_path: String, locked: Arc<AtomicBool>) -> ! {
    println!("[ client ] [ {} ] started in serial mode", name);

    // wait for a new event to write over serial
    loop {
        match read_request(&source_path) {

            //
            Request::TransmitKernel     => {
                locked.store(true, Ordering::Relaxed);
                transmit_file(&name, &source_path, &kernel_path);
                locked.store(false, Ordering::Relaxed);
            },

            //
            Request::TransmitFile(path) => {
                locked.store(true, Ordering::Relaxed);
                transmit_file(&name, &source_path, &path);
                locked.store(false, Ordering::Relaxed);
            },

            //
            Request::Terminate(message) => {
                locked.store(true, Ordering::Relaxed);
                panic!("[ client ] [ {} ] {}. client terminated", name, message)
            },
        }
    }
}
