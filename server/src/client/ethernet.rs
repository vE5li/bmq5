use client::context::*;

use std::net::{ TcpListener, TcpStream, Ipv4Addr };

//
pub struct EthernetClient {
    context:    Context,
}

//
impl EthernetClient {

    //
    pub fn new(context: Context, address: Ipv4Addr) -> Self {
        Self {
            context:    context,
        }
    }
}

//
impl Client for EthernetClient {

    //
    fn start(&mut self) {
        let name = self.context.name.clone();

        if let Some(kernel_path) = &self.context.binary {
            thread::spawn(|| { start(name) });
        }
    }

    //
    #[cfg(feature = "controller")]
    fn event(&mut self, data: u16) {
        // send modifiers if self.context.binary.is_none()
        if self.context.locked.load(Ordering::Relaxed) == false {
            //self.source_file.write(&[(data >> 8) as u8, data as u8]).unwrap();
        }
    }

    //
    #[cfg(feature = "controller")]
    fn index(&self) -> usize {
        self.context.index
    }
}

//
fn start(name: String) -> ! {
    println!("[ client ] [ {} ] started in ethernet mode", name);
    loop {}
}
