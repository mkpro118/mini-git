use std::collections::HashMap;
use std::slice::Iter;

#[derive(Debug, Clone)]
pub enum ArgumentType {
    String,
    Integer,
    Float,
    Boolean,
}

#[derive(Debug, Clone)]
struct Argument {
    name: String,
    short: Option<char>,
    arg_type: ArgumentType,
    required: bool,
    help: String,
}

#[derive(Debug)]
struct SubCommand {
    name: String,
    parser: ArgumentParser,
}

#[derive(Debug)]
pub struct ArgumentParser {
    description: String,
    arguments: Vec<Argument>,
    subcommands: Vec<SubCommand>,
    cmd_chain: Option<String>,
}

#[derive(Debug)]
pub struct Namespace {
    values: HashMap<String, String>,
    subcommand: Option<(String, Box<Namespace>)>,
}

impl Argument {
    pub fn new(
        name: &str,
        short: Option<char>,
        arg_type: ArgumentType,
        required: bool,
        help: &str,
    ) -> Self {
        Argument {
            name: name.to_string(),
            short,
            arg_type,
            required,
            help: help.to_string(),
        }
    }
}

impl SubCommand {
    pub fn new(name: &str, mut parser: ArgumentParser) -> Self {
        parser.cmd_chain = if let Some(prev) = parser.cmd_chain {
            Some(format!("{prev} {name}"))
        } else {
            Some(name.to_owned())
        };

        SubCommand {
            name: name.to_string(),
            parser,
        }
    }
}

impl Namespace {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            subcommand: None,
        }
    }

    pub fn set_subcommand(&mut self, name: &str, namespace: Namespace) {
        self.subcommand = Some((name.to_owned(), Box::new(namespace)))
    }
}

impl Default for Namespace {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ArgumentParser {
    fn default() -> Self {
        Self::new("No description")
    }
}

impl ArgumentParser {
    pub fn new(description: &str) -> Self {
        let mut parser = ArgumentParser {
            description: description.to_string(),
            arguments: Vec::new(),
            subcommands: Vec::new(),
            cmd_chain: None,
        };
        parser.add_argument(
            "help",
            Some('h'),
            ArgumentType::Boolean,
            false,
            "Display this help message",
        );
        parser
    }

    pub fn add_argument(
        &mut self,
        name: &str,
        short: Option<char>,
        arg_type: ArgumentType,
        required: bool,
        help: &str,
    ) {
        self.arguments
            .push(Argument::new(name, short, arg_type, required, help));
    }

    pub fn add_subcommand(&mut self, name: &str, parser: ArgumentParser) {
        self.subcommands.push(SubCommand::new(name, parser));
    }

    pub fn parse_cli(&self) -> Result<Namespace, String> {
        let args = std::env::args().skip(1); //.collect::<Vec<String>>();
        self.parse(args, true)
    }

    pub fn parse_args<'a, 'b>(
        &self,
        args: &'a [&'b str],
    ) -> Result<Namespace, String> {
        self.parse(args.into_iter().map(|&x| x.to_owned()), false)
    }

    fn parse<I>(&self, mut args: I, cli: bool) -> Result<Namespace, String>
    where
        I: Iterator<Item = String>,
    {
        let mut parsed = Namespace::new();

        loop {
            let Some(arg) = args.next() else {
                return Ok(parsed);
            };

            // Check for subcommand
            if let Some(subcommand) =
                self.subcommands.iter().find(|s| s.name == *arg)
            {
                parsed.set_subcommand(
                    &subcommand.name,
                    subcommand.parser.parse(args, cli)?,
                );
                break;
            }

            // Parse arguments
            // Optional arguments
            if arg.starts_with('-') {
                let (find_strategy, err) = if arg.starts_with("--") {
                    let name = &arg[2..];
                    (
                        Box::new(move |a: &&Argument| a.name == name)
                            as Box<dyn Fn(&&Argument) -> bool>,
                        Err(format!("Missing value for argument: {name}")),
                    )
                } else {
                    let short = arg.chars().nth(1).unwrap();
                    (
                        Box::new(move |a: &&Argument| a.short == Some(short))
                            as Box<dyn Fn(&&Argument) -> bool>,
                        Err(format!("Missing value for argument: -{short}")),
                    )
                };

                if let Some(argument) =
                    self.arguments.iter().find(find_strategy)
                {
                    if matches!(argument.arg_type, ArgumentType::Boolean) {
                        parsed
                            .values
                            .insert(argument.name.clone(), "true".to_string());
                    } else {
                        let Some(val) = args.next() else {
                            return err;
                        };
                        parsed.values.insert(argument.name.clone(), val);
                    }
                } else {
                    return Err(format!("Unknown argument: {}", arg));
                }
            } else {
                // Positional argument
                if let Some(argument) = self.arguments.iter().find(|a| {
                    a.required && !parsed.values.contains_key(&a.name)
                }) {
                    parsed.values.insert(argument.name.clone(), arg.clone());
                } else {
                    return Err(format!("Unexpected argument: {}", arg));
                }
            }
        }

        // Check for missing required arguments
        for arg in &self.arguments {
            if arg.required && !parsed.values.contains_key(&arg.name) {
                return Err(format!("Missing required argument: {}", arg.name));
            }
        }

        Ok(parsed)
    }
}
