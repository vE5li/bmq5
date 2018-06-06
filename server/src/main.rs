#[macro_use]
mod parser;
mod client;
#[cfg(feature = "controller")]
mod controller;

// main
fn main() -> ! {

    // get command line arguments
    let mut parameters: Vec<String> = std::env::args().rev().collect();
    parameters.pop();

    // managers
    let client_manager = item!(client::Manager::new());
    #[cfg(feature = "controller")]
    let mut controller_manager = controller::Manager::new();

    // keyboard driver
    #[cfg(feature = "driver")]
    let mut driver = false;

    // devices lookup path
    let mut lookup_path = item!(String::new());

    // file system
    #[cfg(feature = "system")]
    let mut system_path: Option<String> = None;

    //
    {
        //
        let client_names: parser::Item<Vec<String>> = item!(Default::default());
        let mut parser = parser::Parser::new(&parameters.pop().expect("[ server ] no configuration file specified"));

        //
        parser.register("binary", false, Box::new(| stack, prefix | {
            stack.push_debug("no binary file specified");
            client_manager.borrow_mut().binary(format!("{}{}", prefix.unwrap(), stack.pop()), stack.pop_name());
        }));

        //
        parser.register("lookup", true, Box::new(| stack, _ | {
            *lookup_path.borrow_mut() = stack.pop_directory();
        }));

        //
        parser.register("client", false, Box::new(| stack, prefix | {
            stack.push_debug("no client name specified");
            let client_name = stack.pop();
            assert!(client_names.borrow().iter().find(| name | name == &&client_name).is_none(), "[ server ] client names must be unique");
            client_manager.borrow_mut().initialize(&*lookup_path.borrow(), format!("{}{}", prefix.unwrap(), client_name), client_names.borrow().len());
            client_names.borrow_mut().push(client_name);
        }));

        //
        #[cfg(feature = "controller")]
        parser.register("controller", false, Box::new(| stack, prefix | {
            stack.push_debug("no controller translation specified");
            controller_manager.initialize(&client_names.borrow(), format!("{}{}", prefix.unwrap(), stack.pop()));
        }));

        //
        #[cfg(feature = "driver")]
        parser.register("driver", true, Box::new(| stack, _ | {
            driver = stack.pop_state();
        }));

        //
        #[cfg(feature = "system")]
        parser.register("system", true, Box::new(| stack, _ | {
            system_path = Some(stack.pop_directory());
        }));

        //
        parser.parse();
    }

    //
    #[cfg(not(feature = "controller"))]
    unwrap_item!(client_manager).start();
    #[cfg(feature = "controller")]
    controller_manager.start(unwrap_item!(client_manager));
}
