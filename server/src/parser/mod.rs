use std::net::Ipv4Addr;
use std::fs;

pub use std::rc::Rc;
pub use std::cell::RefCell;

//
pub type Item<T> = Rc<RefCell<T>>;

//
macro_rules! item {
    ($item:expr)    => ($crate::parser::Rc::new($crate::parser::RefCell::new($item)));
}

//
macro_rules! unwrap_item {
    ($item:expr)    => ({
        match $crate::parser::Rc::try_unwrap($item) {
            Ok(item)  => item.into_inner(),
            Err(_)    => panic!("[ server ] failed to unwrap item")
        }
    });
}

// get the size of a file in bytes
pub fn file_size(filename: &str) -> u32 {
    match fs::metadata(filename) {
        Ok(metadata)    => metadata.len() as u32,
        Err(_)          => panic!("[ server ] unable to read size of '{}'", filename)
    }
}

//
pub fn checked_path(path: &str, base: &str, extention: &str, reverse: bool) -> Option<String> {
    use std::path::Path;

    //
    let checked_path = format!("{}{}.{}", path, base, extention);
    if Path::new(&checked_path).exists() {
        return Some(checked_path);
    };

    //
    let checked_path = if reverse {
        format!("{}{}/{}", path, extention, base)
    } else {
        format!("{}{}/{}", path, base, extention)
    };

    //
    if Path::new(&checked_path).exists() {
        return Some(checked_path);
    };

    //
    None
}

//
pub fn unwrap_sequence(buffer: &Vec<u8>, sequence: &Vec<usize>) -> u64 {
    let mut value = 0;
    for index in sequence {
        value = value << 8 | buffer[*index] as u64;
    }
    value
}

//
#[derive(Clone)]
pub struct BinaryFile {
    pub path:   String,
    pub name:   Option<String>,
}

// version channels
#[derive(Copy, Clone)]
pub enum Channel {
    Stable,
    Beta,
    Nightly,
    None,
}

// implement channel
impl Channel {

    // get an extention from the channel
    pub fn suffix(&self) -> &'static str {
        match *self {
            Channel::Stable     => ".stable",
            Channel::Beta       => ".beta",
            Channel::Nightly    => ".nightly",
            Channel::None       => "",
        }
    }
}

//
pub enum CallMode {
    Item(String),
    Once(bool),
}

//
impl CallMode {

    //
    pub fn new(once: bool) -> Self {
        match once {
            false   => CallMode::Item(String::new()),
            true    => CallMode::Once(false),
        }
    }
}

//
pub type Function<'a> = Box<FnMut(&mut Stack, Option<&str>) + 'a>;

//
pub struct Keyword<'a> {
    pub identifier:     String,
    pub call_mode:      CallMode,
    pub function:       Function<'a>,
}

//
pub struct Stack {
    lines:              Vec<Option<Vec<String>>>,
    debug_stack:        Vec<String>,
    current_line:       usize,
    counter:            usize,
}

//
impl Stack {

    //
    pub fn new(path: &str) -> Self {
        use std::io::{ BufRead, BufReader };

        match fs::File::open(path) {
            Ok(file)    => {

                // collect all words in the file
                let mut lines = Vec::new();
                for line in BufReader::new(file).lines() {
                    if let Ok(line) = line {
                        let line = if line.len() > 0 && line.chars().nth(0).unwrap() != '#' {
                            Some(line.split_whitespace().map(| word | String::from(word)).rev().collect())
                        } else {
                            None
                        };
                        lines.insert(0, line);
                    }
                }
                lines.push(None);

                // return new parser
                Self {
                    lines:          lines,
                    debug_stack:    Vec::new(),
                    current_line:   0,
                    counter:        0,
                }
            },
            Err(_)      => panic!("[ parser ] unable to open '{}'", path)
        }
    }

    //
    pub fn panic(&self, message: String) -> ! {
        panic!("[ parser ] [ line : {} ] {}", self.current_line, message)
    }

    //
    pub fn push_debug(&mut self, debug_message: &str) {
        self.debug_stack.push(String::from(debug_message));
    }

    //
    pub fn pop(&mut self) -> String {
        let debug_message = self.debug_stack.pop().unwrap();
        match self.lines.last_mut().unwrap() {
            Some(words) => {
                match words.pop() {
                    Some(word)  => word,
                    None        => panic!("[ parser ] [ line : {} ] {}", self.current_line, debug_message)
                }
            },
            None        => panic!("[ parser ] invalid line")
        }
    }

    //
    pub fn pop_newline(&mut self) -> Option<String> {
        while let Some(line) = self.lines.pop() {
            self.current_line += 1;
            if self.lines.last()?.is_some() {
                break
            }
        };
        self.push_debug("no key specified");
        Some(self.pop())
    }

    //
    pub fn pop_directory(&mut self) -> String {
        self.push_debug("no directory specified");
        let mut directory = self.pop();
        if directory.chars().rev().nth(0).unwrap() != '/' {
            directory.push('/');
        };
        directory
    }

    //
    pub fn pop_counter(&mut self) -> usize {
        self.push_debug("no value specified");
        match self.pop().as_ref() {
            "*"     => {
                self.counter += 1;
                self.counter
            },
            value   => {
                match value.parse() {
                    Ok(value)   => {
                        self.counter = value;
                        value
                    },
                    Err(_)      => panic!("[ parser ] [ line : {} ] failed to parse value", self.current_line)
                }
            },
        }
    }

    //
    pub fn pop_u64(&mut self) -> u64 {
        self.push_debug("no number specified");
        match self.pop().parse() {
            Ok(number)  => number,
            Err(_)      => panic!("[ parser ] [ line : {} ] failed to parse number", self.current_line)
        }
    }

    //
    pub fn pop_usize(&mut self) -> usize {
        self.pop_u64() as usize
    }

    //
    pub fn pop_u8(&mut self) -> u8 {
        self.pop_u64() as u8
    }

    //
    pub fn pop_mode(&mut self) -> Option<u8> {
        self.push_debug("no mode specified");
        match self.pop().as_ref() {
            "base"  => None,
            word    => {
                if word.len() != 8 {
                    self.panic(format!("mode identifier must have 8 bits"));
                }
                let mut mask = 0;
                for offset in 0..8 {
                    match word.chars().nth(offset).unwrap() {
                        '0'     => continue,
                        '1'     => mask |= 1 << offset,
                        state   => self.panic(format!("unexpected character '{}' in channel identifier", state))
                    }
                }
                Some(mask)
            },
        }
    }

    //
    pub fn pop_index(&mut self) -> usize {
        self.push_debug("no index specified");
        self.pop().parse().expect("unable to parse index")
    }

    //
    pub fn pop_state(&mut self) -> bool {
        self.push_debug("no state specified. valid options are 'enabled' and 'disabled'");
        match self.pop().as_ref() {
            "enabled"   => true,
            "disabled"  => false,
            state       => panic!("[ parser ] invalid state '{}'. valid options are 'enabled' and 'disabled'", state)
        }
    }

    //
    pub fn pop_channel(&mut self) -> Channel {
        self.push_debug("no channel specified. valid options are 'stable', 'beta', 'nightly' or 'none'");
        match self.pop().as_ref() {
            "stable"    => Channel::Stable,
            "beta"      => Channel::Beta,
            "nightly"   => Channel::Nightly,
            "none"      => Channel::None,
            channel     => panic!("[ parser ] invalid channel '{}'. valid options are 'stable', 'beta', 'nightly' or 'none'", channel)
        }
    }

    //
    pub fn pop_ip(&mut self) -> Ipv4Addr {
        self.push_debug("no ip address specified");
        self.pop().parse().expect("[ parser ] unable to parse ip address")
    }

    //
    pub fn pop_name(&mut self) -> Option<String> {
        self.push_debug("no name specified");
        let name = self.pop();
        match name.as_ref() {
            "!"     => None,
            _       => Some(name),
        }
    }

    //
    pub fn pop_binary(&mut self, binary_files: &Vec<BinaryFile>) -> String {

        //
        self.push_debug("no binary name or index specified");
        let word = self.pop();

        //
        match word.parse() {
            Ok(index)    => {
                let index: usize = index;
                binary_files[index].path.clone()
            },
            _            => {
                for (index, binary) in binary_files.iter().enumerate() {
                    if let Some(name) = &binary.name {
                        if name == &word {
                            return binary.path.clone()
                        }
                    }
                }
                panic!("[ parser ] binary name '{}' not found", word)
            },
        }
    }

    //
    pub fn pop_ascii(&mut self) -> u8 {
        self.push_debug("no character specified");
        let word = self.pop();
        match word.chars().nth(0).unwrap() {
            'b'     => word.chars().nth(1).expect("[ parser ] no character specified") as u8,
            's'     => 32,
            _       => word.parse().expect("[ parser ] failed to parse character"),
        }
    }

    //
    pub fn pop_sequence(&mut self) -> Vec<usize> {
        match self.lines.last_mut().unwrap() {
            Some(words) => {

                //
                let mut sequence: Vec<usize> = Vec::new();
                while let Some(word) = words.pop() {

                    //
                    if word == ";" {
                        match sequence.len() {
                            0       => panic!("[ parser ] [ line : {} ] empty sequence", self.current_line),
                            1...3   => return sequence,
                            _       => panic!("[ parser ] [ line : {} ] sequence must be 4 items at max", self.current_line),
                        }
                    }

                    //
                    match word.parse() {
                        Ok(number)  => sequence.push(number),
                        Err(_)      => panic!("[ parser ] [ line : {} ] failed to parse sequence", self.current_line)
                    }
                }
                panic!("[ parser ] [ line : {} ] unterminated sequence", self.current_line)
            },
            None        => panic!("[ parser ] invalid line")
        }
    }

    //
    #[cfg(feature = "controller")]
    pub fn pop_client(&mut self, client_names: &Vec<String>) -> u8 {

        //
        self.push_debug("no client name or index specified");
        let word = self.pop();

        //
        match word.parse() {
            Ok(index)   => index,
            _           => {
                for (index, name) in client_names.iter().enumerate() {
                    if name == &word {
                        return index as u8
                    }
                }
                panic!("[ parser ] client name not found")
            },
        }
    }
}

//
pub struct Parser<'a> {
    stack:      Stack,
    keywords:   Vec<Keyword<'a>>,
}

//
impl<'a> Parser<'a> {

    //
    pub fn new(path: &str) -> Self {
        Self {
            stack:    Stack::new(path),
            keywords:       Default::default(),
        }
    }

    //
    pub fn register(&mut self, identifier: &str, once: bool, function: Function<'a>) {
        self.keywords.push(Keyword {
            identifier:     String::from(identifier),
            function:       function,
            call_mode:      CallMode::new(once),
        });
    }

    //
    pub fn parse(&mut self) {

        //
        while let Some(mut word) = self.stack.pop_newline() {

            //
            let call_identifier = word.remove(0);

            //
            let keyword = self.keywords.iter_mut().find(| keyword | keyword.identifier == word);
            let mut keyword = match keyword {
                Some(keyword)   => keyword,
                None            => self.stack.panic(format!("invalid keyword '{}'", word)),
            };

            //
            match call_identifier {

                //
                '?'         => {
                    match keyword.call_mode {
                        CallMode::Once(invalid) => {
                            match invalid {
                                false   => {
                                    (keyword.function)(&mut self.stack, None);
                                    keyword.call_mode = CallMode::Once(true)
                                },
                                true    => self.stack.panic(format!("repeated call of call once")),
                            }
                        },
                        _                       => self.stack.panic(format!("invalid call mode. use ':', '+' or '@' for '{}'", keyword.identifier)),
                    }
                },

                //
                '@'         => {
                    match &mut keyword.call_mode {
                        CallMode::Item(prefix)  => {
                            self.stack.push_debug("no prefix specified");
                            *prefix = self.stack.pop();
                        },
                        _                       => self.stack.panic(format!("invalid call mode. use '$' for '{}'", keyword.identifier)),
                    }
                },

                //
                ':' | '+'   => {
                    match &keyword.call_mode {
                        CallMode::Item(prefix)  => (keyword.function)(&mut self.stack, Some(prefix)),
                        _                       => self.stack.panic(format!("invalid call mode. use '$' for '{}'", keyword.identifier)),
                    }
                },

                //
                mode    => self.stack.panic(format!("invalid call mode '{}'", mode)),
            };
        }
    }
}
