use std::collections::HashMap;

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
        let args = std::env::args().skip(1).collect::<Vec<String>>();
        self.parse(&args, true)
    }

    pub fn parse_args(&self, args: &[String]) -> Result<Namespace, String> {
        self.parse(args, false)
    }

    fn parse(&self, _args: &[String], _cli: bool) -> Result<Namespace, String> {
        todo!()
    }
}
