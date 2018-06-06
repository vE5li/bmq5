mod event;

use controller::event::{ Event, Action, Rules };
use std::sync::mpsc::{ channel, Sender, Receiver };
use std::time::Duration;
use std::thread;
use client;

//
pub struct Context {
    sender:         Sender<u32>,
    events:         Vec<Event>,
    bytes_event:    Vec<usize>,
    source:         String,
    width:          usize,
    verbose:        bool,
}

//
impl Context {

    //
    pub fn new(sender: Sender<u32>, translation_path: String, client_names: &Vec<String>) -> (Self, u8) {
        use parser::{ Parser, checked_path };

        //
        let mut rules = item!(Rules::new());
        let mut events: Vec<Event> = Vec::new();
        let mut bytes_event: Vec<usize> = Vec::new();
        let mut source: Option<String> = None;
        let mut width: Option<usize> = None;
        let mut verbose = false;
        let mut target = 0;

        {
            //
            let mut parser: Parser = Parser::new(&translation_path);

            //
            parser.register("source", true, Box::new(| stack, _ | {
                stack.push_debug("no source name specified");
                source = Some(stack.pop());
            }));

            //
            parser.register("verbose", true, Box::new(| stack, _ | {
                verbose = stack.pop_state();
            }));

            //
            parser.register("target", true, Box::new(| stack, _ | {
                target = stack.pop_client(client_names);
            }));

            //
            parser.register("width", true, Box::new(| stack, _ | {
                width = Some(stack.pop_usize());
            }));

            //
            parser.register("event", false, Box::new(| stack, prefix | {
                stack.push_debug("no event name specified");

                //
                let index = translation_path.len() - translation_path.chars().rev().position(| character | character == '/').unwrap();
                let path = match checked_path(&translation_path[..index], &format!("{}{}", prefix.unwrap(), stack.pop()), "event", true) {
                    Some(path)  => path,
                    None        => panic!("[ controller ] failed to find event path")
                };

                //
                let identifier = stack.pop_u64();
                assert!(events.iter().find(| event | event.identifier == identifier).is_none(), "[ controller ] event identifier must be unique");
                events.push(Event::new(rules.borrow().clone(), path, identifier, client_names));
            }));

            //
            parser.register("byte", false, Box::new(| stack, prefix | {
                stack.push_debug("no byte rule specified");
                match stack.pop().as_ref() {
                    "event"     => bytes_event = stack.pop_sequence(),
                    "id"        => rules.borrow_mut().bytes_id = Some(stack.pop_sequence()),
                    "value"     => rules.borrow_mut().bytes_value = Some(stack.pop_sequence()),
                    rule        => panic!("[ controller ] no byte rule called '{}'", rule)
                }
            }));

            //
            parser.register("value", false, Box::new(| stack, prefix | {
                stack.push_debug("no value rule specified");
                match stack.pop().as_ref() {
                    "up"        => rules.borrow_mut().value_up = Some(stack.pop_u64()),
                    "down"      => rules.borrow_mut().value_down = Some(stack.pop_u64()),
                    "repeat"    => rules.borrow_mut().value_repeat = Some(stack.pop_u64()),
                    "center"    => rules.borrow_mut().value_center = Some(stack.pop_u64()),
                    rule        => panic!("[ controller ] no value rule called '{}'", rule)
                }
            }));

            //
            parser.parse();
        }

        //
        (Self {
            sender:         sender,
            events:         events,
            bytes_event:    bytes_event,
            source:         source.expect("[ controller ] no source file specified"),
            width:          width.expect("[ controller ] no event width specified"),
            verbose:        verbose,
        },
        target)
    }

    //
    pub fn start(self, target: u8) {
        thread::spawn(move || { start_controller(self, target) });
    }
}

//
pub struct Manager {
    sender:         Sender<u32>,
    receiver:       Receiver<u32>,
}

//
impl Manager {

    //
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender:     sender,
            receiver:   receiver,
        }
    }

    //
    pub fn initialize(&mut self, client_names: &Vec<String>, translation_path: String) {
        let (context, target) = Context::new(self.sender.clone(), translation_path, client_names);
        context.start(target);
    }

    //
    pub fn start(self, client_manager: client::Manager) -> ! {
        use client::Client;

        // get the clients from the client manager
        let (mut serial_clients, mut ethernet_clients) = client_manager.clients();

        // setup vector for collected clients
        let mut clients: Vec<&mut client::Client> = Default::default();
        let length = serial_clients.len() + ethernet_clients.len();
        clients.reserve(length);
        unsafe { clients.set_len(length) };

        // fill clients vector and start each client
        serial_clients.iter_mut().for_each(| client | { let index = client.index(); clients[index] = client });
        ethernet_clients.iter_mut().for_each(| client | { let index = client.index(); clients[index] = client });
        clients.iter_mut().for_each(| client | client.start());

        // event loop
        loop {
            let data = self.receiver.recv().unwrap();
            clients[(data >> 24) as usize].event(data as u16);
        }
    }
}

//
pub fn start_controller(context: Context, mut target: u8) -> ! {
    use parser::unwrap_sequence;
    use std::fs::File;
    use std::io::Read;

    //
    let mut modifiers: u8 = 0;
    let mut buffer: Vec<u8> = Vec::with_capacity(context.width);
    unsafe { buffer.set_len(context.width) };

    //
    loop {
        if let Ok(mut source_file) = File::open(&context.source) {
            while let Ok(_) = source_file.read_exact(&mut buffer) {
                if context.verbose {
                    println!("\n[ controller ] event buffer: {:?}", &buffer);
                }

                //
                let event_identifier = unwrap_sequence(&buffer, &context.bytes_event);
                if context.verbose {
                    println!("[ controller ] event number: {}", event_identifier);
                }

                //
                if let Some(event) = context.events.iter().find(| event | event.identifier == event_identifier) {
                    if let Some(data) = event.translate(&buffer, &mut target, &mut modifiers, context.verbose) {
                        if context.verbose {
                            println!("[ controller ] character sent: {}", data);
                        }
                        let combined = (target as u32) << 24 | (modifiers as u32) << 8 | (data as u32);
                        context.sender.send(combined).unwrap();
                    }
                } else if context.verbose {
                    println!("[ controller ] unhandeled event: {}", event_identifier);
                }
            }
        } else {
            thread::sleep(Duration::new(2, 0));
        }
    }
}
