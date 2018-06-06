pub use std::sync::Arc;
pub use std::sync::atomic::{ AtomicBool, Ordering };
pub use parser::{ file_size, checked_path };
pub use client::Client;
pub use std::thread;

use parser::{ Parser, Channel, BinaryFile, Item };
use std::net::Ipv4Addr;
use std::fs;

//
pub enum Request {
    TransmitKernel,
    TransmitFile(String),
    Terminate(String),

}

//
pub enum Mode {
    Serial(String),
    Ethernet(Ipv4Addr),
}

//
pub struct Context {
    pub name:       String,
    pub binary:     Option<String>,
    pub index:      usize,
    pub locked:     Arc<AtomicBool>,
}

//
impl Context {

    //
    pub fn new(binary_files: &Vec<BinaryFile>, name: String, translation_path: String, index: usize) -> (Self, Mode) {
        let mut channel = Channel::Stable;
        let mut mode: Item<Option<Mode>> = item!(None);
        let mut binary: Option<String> = None;

        //
        {
            let mut parser = Parser::new(&translation_path);

            parser.register("serial", true, Box::new(| stack, _ | {
                stack.push_debug("no serial source specified");
                *mode.borrow_mut() = Some(Mode::Serial(stack.pop()));
            }));

            parser.register("ethernet", true, Box::new(| stack, _ | {
                *mode.borrow_mut() = Some(Mode::Ethernet(stack.pop_ip()));
            }));

            parser.register("channel", true, Box::new(| stack, _ | {
                channel = stack.pop_channel();
            }));

            parser.register("use", true, Box::new(| stack, _ | {
                binary = Some(stack.pop_binary(binary_files));
            }));

            parser.parse();
        }

        // set the release channel
        if let Some(prefix) = binary {
            binary = Some(prefix + &channel.suffix());
        }

        //
        (Self {
            name:           name,
            binary:         binary,
            index:          index,
            locked:         Arc::new(AtomicBool::new(false)),
        },
        unwrap_item!(mode).expect("[ server ] no client mode specified"))
    }
}
