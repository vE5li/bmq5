mod serial;
mod ethernet;
mod context;

pub use self::serial::SerialClient;
pub use self::ethernet::EthernetClient;
pub use self::context::{ Context, Mode };

use parser::BinaryFile;

// generic client
pub trait Client {

    // start the device
    fn start(&mut self);

    // send an event to the device
    #[cfg(feature = "controller")]
    fn event(&mut self, data: u16);

    // get the client index
    #[cfg(feature = "controller")]
    fn index(&self) -> usize;
}

// client manager
pub struct Manager {
    serial_clients:     Vec<SerialClient>,
    ethernet_clients:   Vec<EthernetClient>,
    binary_files:       Vec<BinaryFile>,
}

// implement client manager
impl Manager {

    // create a new client manager
    pub fn new() -> Self {
        Self {
            serial_clients:         Default::default(),
            ethernet_clients:       Default::default(),
            binary_files:           Default::default(),
        }
    }

    // add a binary file that can be loaded
    pub fn binary(&mut self, path: String, name: Option<String>) {
        assert!(self.binary_files.iter().find(| binary | binary.path == path).is_none(), "[ server ] binary paths must me unique");

        // if the binary file has a special name, make sure it's unique
        if let Some(name) = &name {
            assert!(self.binary_files.iter().find(| binary | {

                // compare the binary name with the new name
                match &binary.name {
                    Some(inner) => inner == name,
                    None        => false,
                }
            }).is_none(), "[ server ] binary names must me unique");
        }

        // store the new binary
        self.binary_files.push(BinaryFile { path: path, name: name });
    }

    //
    pub fn initialize(&mut self, lookup_path: &str, name: String, index: usize) {
        use parser::checked_path;

        //
        let path = match context::checked_path(lookup_path, &name, "client", false) {
            Some(path)  => path,
            None        => panic!("[ server ] [ {} ] failed to find configuration path", name)
        };


        let (context, mode) = Context::new(&self.binary_files, name, path, index);
        println!("[ server ] [ {} ] client initialized", context.name);

        //
        match mode {
            Mode::Serial(source_path)   => self.serial_clients.push(SerialClient::new(context, source_path)),
            Mode::Ethernet(ip_address)  => self.ethernet_clients.push(EthernetClient::new(context, ip_address)),
        }
    }

    //
    pub fn clients(self) -> (Vec<SerialClient>, Vec<EthernetClient>) {
        (self.serial_clients, self.ethernet_clients)
    }

    //
    #[cfg(not(feature = "controller"))]
    pub fn start(mut self) -> ! {
        self.serial_clients.iter_mut().for_each(| client | client.start());
        self.ethernet_clients.iter_mut().for_each(| client | client.start());
        loop {}
    }
}
