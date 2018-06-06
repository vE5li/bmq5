use std::sync::mpsc::Sender;
use parser::Stack;

//
const TRANSLATION_SIZE: usize       = 128;

//
pub type Field = u16;

//
#[derive(Copy, Clone)]
pub enum Action {

    // character, down, repeat
    Press(u8),

    // mode bit offset, down
    Toggle(u8),

    // mode bit offset, up, down
    Set(u8),

    // client index
    Target(u8),

    // modifier index
    Push(u8),

    //
    None,
}

//
pub type Value = Option<u64>;
pub type Sequence = Option<Vec<usize>>;

//
#[derive(Clone)]
pub struct Rules {
     pub bytes_id:      Sequence,
     pub bytes_value:   Sequence,
     pub value_up:      Value,
     pub value_down:    Value,
     pub value_repeat:  Value,
     pub value_center:  Value,
}

//
impl Rules {

    //
    pub fn new() -> Self {
        Self {
            bytes_id:       None,
            bytes_value:    None,
            value_up:       None,
            value_down:     None,
            value_repeat:   None,
            value_center:   None,
        }
    }
}

//
pub type Translation = [Action; TRANSLATION_SIZE];

//
pub struct Mode {
    pub translation:    Translation,
    pub mask:           u8,
}

//
impl Mode {

    //
    pub fn new(translation: Translation, mask: u8) -> Self {
        Self {
            translation:    translation,
            mask:           mask,
        }
    }
}

//
pub struct Event {
    pub modes:          Vec<Mode>,
    pub base_mode:   Option<Mode>,
    pub rules:          Rules,
    pub identifier:     u64,
}

//
impl Event {

    //
    pub fn new(rules: Rules, event_path: String, identifier: u64, client_names: &Vec<String>) -> Self {
        use parser::{ Parser, Item };

        //
        let mut modes: Item<Vec<Mode>> = item!(Vec::new());
        let mut base_mode: Item<Option<Mode>> = item!(None);
        let mut rules = item!(rules);

        {
            //
            let mut parser = Parser::new(&event_path);

            //
            parser.register("byte", false, Box::new(| stack, _ | {
                stack.push_debug("no byte rule specified");
                match stack.pop().as_ref() {
                    "id"        => rules.borrow_mut().bytes_id = Some(stack.pop_sequence()),
                    "value"     => rules.borrow_mut().bytes_value = Some(stack.pop_sequence()),
                    rule        => panic!("[ controller ] no byte rule called '{}'", rule)
                }
            }));

            //
            parser.register("value", false, Box::new(| stack, _ | {
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
            parser.register("mode", false, Box::new(| stack, _ | {
                match stack.pop_mode() {
                    Some(mask)  => {
                        let translation = match *base_mode.borrow() {
                            Some(ref mode)  => mode.translation.clone(),
                            None        => [Action::None; TRANSLATION_SIZE],
                        };
                        modes.borrow_mut().push(Mode::new(translation, mask))
                    },
                    None        => *base_mode.borrow_mut() = Some(Mode::new([Action::None; TRANSLATION_SIZE], 0)),
                }
            }));

            //
            parser.register("press", false, Box::new(| stack, _ | {
                let index = stack.pop_counter();
                let action = Action::Press(stack.pop_ascii());
                match modes.borrow_mut().last_mut() {
                    Some(mode)  => mode.translation[index] = action, // TODO: assure it's unique
                    None        => {
                        match *base_mode.borrow_mut() {
                            Some(ref mut mode)  => mode.translation[index] = action,
                            None        => panic!("[ controller ] no mode initialized"),
                        }
                    },
                };
            }));

            //
            parser.register("toggle", false, Box::new(| stack, _ | {
                let index = stack.pop_counter();
                let action = Action::Toggle(stack.pop_u8());
                match modes.borrow_mut().last_mut() {
                    Some(mode)  => mode.translation[index] = action, // TODO: assure it's unique
                    None        => {
                        match *base_mode.borrow_mut() {
                            Some(ref mut mode)  => mode.translation[index] = action,
                            None        => panic!("[ controller ] no mode initialized"),
                        }
                    },
                };
            }));

            //
            parser.register("set", false, Box::new(| stack, _ | {
                let index = stack.pop_counter();
                let action = Action::Set(stack.pop_u8());
                match modes.borrow_mut().last_mut() {
                    Some(mode)  => mode.translation[index] = action, // TODO: assure it's unique
                    None        => {
                        match *base_mode.borrow_mut() {
                            Some(ref mut mode)  => mode.translation[index] = action,
                            None        => panic!("[ controller ] no mode initialized"),
                        }
                    },
                };
            }));

            //
            parser.register("target", false, Box::new(| stack, _ | {
                let index = stack.pop_counter();
                let action = Action::Target(stack.pop_client(client_names));
                match modes.borrow_mut().last_mut() {
                    Some(mode)  => mode.translation[index] = action, // TODO: assure it's unique
                    None        => {
                        match *base_mode.borrow_mut() {
                            Some(ref mut mode)  => mode.translation[index] = action,
                            None        => panic!("[ controller ] no mode initialized"),
                        }
                    },
                };
            }));

            //
            parser.register("push", false, Box::new(| stack, _ | {
                let index = stack.pop_counter();
                let action = Action::Push(stack.pop_u8());
                match modes.borrow_mut().last_mut() {
                    Some(mode)  => mode.translation[index] = action, // TODO: assure it's unique
                    None        => {
                        match *base_mode.borrow_mut() {
                            Some(ref mut mode)  => mode.translation[index] = action,
                            None        => panic!("[ controller ] no mode initialized"),
                        }
                    },
                };
            }));

            parser.parse();
        }

        //
        assert!(rules.borrow().bytes_id.is_some(), "[ controller ] no bytes for id specified");

        //
        Self {
            modes:          unwrap_item!(modes),
            base_mode:      unwrap_item!(base_mode),
            rules:          unwrap_item!(rules),
            identifier:     identifier,
        }
    }

    //
    pub fn translate(&self, buffer: &Vec<u8>, target: &mut u8, modifiers: &mut u8, verbose: bool) -> Option<u8> {
        use parser::unwrap_sequence;

        if let Some(bytes_id) = &self.rules.bytes_id {

            //
            let bytes_id = unwrap_sequence(buffer, bytes_id);
            if verbose {
                println!("[ controller ] key identifier: {}", bytes_id);
            }

            let mode = match self.modes.iter().find(| mode | mode.mask == *modifiers) {
                Some(mode)  => mode,
                None        => match &self.base_mode {
                    Some(mode)  => mode,
                    None        => {
                        if verbose {
                            println!("[ controller ] unhandeled mode: {:b}", modifiers);
                        }
                        return None
                    },
                },
            };

            match mode.translation[bytes_id as usize] {

                //
                Action::Press(character)    => {
                    if let Some(bytes_value) = &self.rules.bytes_value {
                        let bytes_value = unwrap_sequence(buffer, bytes_value);

                        //
                        if let Some(value_down) = self.rules.value_down {
                            if bytes_value == value_down {
                                return Some(character)
                            }
                        }

                        //
                        if let Some(value_repeat) = self.rules.value_repeat {
                            if bytes_value == value_repeat {
                                return Some(character)
                            }
                        }
                    }
                },

                // TODO: check offset bounds
                Action::Set(offset)         => {
                    if let Some(bytes_value) = &self.rules.bytes_value {
                        let bytes_value = unwrap_sequence(buffer, bytes_value);

                        //
                        if let Some(value_down) = self.rules.value_down {
                            if bytes_value == value_down {
                                *modifiers |= 1 << offset;
                            }
                        }

                        //
                        if let Some(value_up) = self.rules.value_up {
                            if bytes_value == value_up {
                                *modifiers &= !(1 << offset);
                            }
                        }
                    }
                },

                //
                Action::Toggle(offset)      => {
                    if let Some(bytes_value) = &self.rules.bytes_value {
                        let bytes_value = unwrap_sequence(buffer, bytes_value);

                        //
                        if let Some(value_down) = self.rules.value_down {
                            if bytes_value == value_down {
                                *modifiers ^= 1 << offset;
                            }
                        }
                    }
                },

                //
                Action::Target(new_target)  => {
                    if let Some(bytes_value) = &self.rules.bytes_value {
                        let bytes_value = unwrap_sequence(buffer, bytes_value);

                        //
                        if let Some(value_down) = self.rules.value_down {
                            if bytes_value == value_down {
                                *target = new_target;
                                println!("[ controller ] switched target to {}", target);
                            }
                        }
                    }
                },

                //
                Action::Push(offset)        => {
                    if let Some(bytes_value) = &self.rules.bytes_value {
                        let bytes_value = unwrap_sequence(buffer, bytes_value);

                        //
                        if let Some(value_center) = self.rules.value_center {
                            if bytes_value < value_center {
                                *modifiers |= 1 << offset;
                            } else {
                                *modifiers &= !(1 << offset);
                            }
                        }
                    }
                },

                //
                _           => return None
            }
        }
        None
    }
}
