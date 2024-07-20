use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ArgumentType {
    String,
    Integer,
    Float,
    Boolean,
}

#[derive(Debug)]
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
                break;
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
                    if argument.name == "help" {
                        if cli {
                            println!("{}", self.help());
                            std::process::exit(0);
                        } else {
                            parsed.values.clear();
                            parsed.values.insert(argument.name.clone(), arg);
                            return Ok(parsed);
                        }
                    }

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

    pub fn help(&self) -> String {
        let name = std::env::args()
            .into_iter()
            .next()
            .expect("executable name");
        let mut help_text = format!(
            "{name}\n{}\n\nUsage: {name} {} [OPTIONS]",
            self.description,
            self.cmd_chain.as_ref().map_or("", |x| x.as_str())
        );

        if !self.subcommands.is_empty() {
            help_text.push_str(" [SUBCOMMAND]");
        }

        help_text.push_str("\n\nOptions:\n");

        for arg in &self.arguments {
            let short = arg
                .short
                .map_or_else(|| " ".repeat(4), |c| format!("-{}, ", c));
            let required = if arg.required { " (required)" } else { "" };
            help_text.push_str(&format!(
                "  {}--{:<20}{} {}\n",
                short, arg.name, arg.help, required
            ));
        }

        if !self.subcommands.is_empty() {
            help_text.push_str("\nSubcommands:\n");
            for subcommand in &self.subcommands {
                help_text.push_str(&format!(
                    "  {:<20} {}\n",
                    subcommand.name, subcommand.parser.description
                ));
            }
        }

        help_text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a basic ArgumentParser
    fn create_basic_parser() -> ArgumentParser {
        let mut parser = ArgumentParser::new("Test parser");
        parser.add_argument(
            "name",
            Some('n'),
            ArgumentType::String,
            true,
            "Name",
        );
        parser.add_argument(
            "age",
            Some('a'),
            ArgumentType::Integer,
            false,
            "Age",
        );
        parser
    }

    #[test]
    fn test_argument_creation() {
        let arg = Argument::new(
            "test",
            Some('t'),
            ArgumentType::String,
            true,
            "Test arg",
        );
        assert_eq!(arg.name, "test");
        assert_eq!(arg.short, Some('t'));
        assert!(matches!(arg.arg_type, ArgumentType::String));
        assert!(arg.required);
        assert_eq!(arg.help, "Test arg");
    }

    #[test]
    fn test_subcommand_creation() {
        let parser = ArgumentParser::new("Sub parser");
        let subcommand = SubCommand::new("sub", parser);
        assert_eq!(subcommand.name, "sub");
        assert_eq!(subcommand.parser.cmd_chain, Some("sub".to_string()));
    }

    #[test]
    fn test_namespace_creation() {
        let mut ns = Namespace::new();
        assert!(ns.values.is_empty());
        assert!(ns.subcommand.is_none());

        ns.set_subcommand("test", Namespace::new());
        assert!(ns.subcommand.is_some());
        assert_eq!(ns.subcommand.as_ref().unwrap().0, "test");
    }

    #[test]
    fn test_argument_parser_creation() {
        let parser = ArgumentParser::new("Test parser");
        assert_eq!(parser.description, "Test parser");
        assert_eq!(parser.arguments.len(), 1); // Should have default --help argument
        assert!(parser.subcommands.is_empty());
        assert!(parser.cmd_chain.is_none());
    }

    #[test]
    fn test_add_argument() {
        let mut parser = ArgumentParser::new("Test parser");
        parser.add_argument(
            "test",
            Some('t'),
            ArgumentType::String,
            true,
            "Test arg",
        );
        assert_eq!(parser.arguments.len(), 2); // Including default --help
        let arg = &parser.arguments[1];
        assert_eq!(arg.name, "test");
        assert_eq!(arg.short, Some('t'));
        assert!(matches!(arg.arg_type, ArgumentType::String));
        assert!(arg.required);
        assert_eq!(arg.help, "Test arg");
    }

    #[test]
    fn test_add_subcommand() {
        let mut parser = ArgumentParser::new("Main parser");
        let sub_parser = ArgumentParser::new("Sub parser");
        parser.add_subcommand("sub", sub_parser);
        assert_eq!(parser.subcommands.len(), 1);
        assert_eq!(parser.subcommands[0].name, "sub");
        assert_eq!(parser.subcommands[0].parser.description, "Sub parser");
        assert_eq!(
            parser.subcommands[0].parser.cmd_chain,
            Some("sub".to_string())
        );
    }

    #[test]
    fn test_parse_args_basic() {
        let parser = create_basic_parser();
        let result = parser.parse_args(&["--name", "John", "--age", "30"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("name"), Some(&"John".to_string()));
        assert_eq!(namespace.values.get("age"), Some(&"30".to_string()));
    }

    #[test]
    fn test_parse_args_missing_required() {
        let parser = create_basic_parser();
        let result = parser.parse_args(&["--age", "30"]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing required argument: name");
    }

    #[test]
    fn test_parse_args_unknown_argument() {
        let parser = create_basic_parser();
        let result =
            parser.parse_args(&["--name", "John", "--unknown", "value"]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Unknown argument: --unknown");
    }

    #[test]
    fn test_parse_args_boolean_flag() {
        let mut parser = create_basic_parser();
        parser.add_argument(
            "flag",
            Some('f'),
            ArgumentType::Boolean,
            false,
            "Flag",
        );
        let result = parser.parse_args(&["--name", "John", "--flag"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("flag"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_args_short_options() {
        let parser = create_basic_parser();
        let result = parser.parse_args(&["-n", "John", "-a", "30"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("name"), Some(&"John".to_string()));
        assert_eq!(namespace.values.get("age"), Some(&"30".to_string()));
    }

    #[test]
    fn test_parse_args_missing_value() {
        let parser = create_basic_parser();
        let result = parser.parse_args(&["--name"]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing value for argument: name");
    }

    #[test]
    fn test_parse_args_with_subcommand() {
        let mut parser = create_basic_parser();
        let mut sub_parser = ArgumentParser::new("Sub parser");
        sub_parser.add_argument(
            "sub_arg",
            Some('s'),
            ArgumentType::String,
            true,
            "Sub arg",
        );
        parser.add_subcommand("sub", sub_parser);

        let result =
            parser.parse_args(&["--name", "John", "sub", "--sub_arg", "value"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("name"), Some(&"John".to_string()));
        assert!(namespace.subcommand.is_some());
        let (sub_name, sub_ns) = namespace.subcommand.unwrap();
        assert_eq!(sub_name, "sub");
        assert_eq!(sub_ns.values.get("sub_arg"), Some(&"value".to_string()));
    }

    #[test]
    fn test_help_output() {
        let parser = create_basic_parser();
        let help_text = parser.help();
        assert!(help_text.contains("Test parser"));
        assert!(help_text.contains("--name"));
        assert!(help_text.contains("--age"));
        assert!(help_text.contains("-n"));
        assert!(help_text.contains("-a"));
        assert!(help_text.contains("(required)"));
    }

    #[test]
    fn test_help_with_subcommands() {
        let mut parser = create_basic_parser();
        let sub_parser = ArgumentParser::new("Sub parser");
        parser.add_subcommand("sub", sub_parser);
        let help_text = parser.help();
        assert!(help_text.contains("Subcommands:"));
        assert!(help_text.contains("sub"));
        assert!(help_text.contains("Sub parser"));
    }

    #[test]
    fn test_parse_args_help_flag() {
        let parser = create_basic_parser();
        let result = parser.parse_args(&["--help"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("help"), Some(&"--help".to_string()));
        assert!(namespace.values.len() == 1);
    }

    #[test]
    fn test_parse_args_unexpected_positional() {
        let parser = create_basic_parser();
        let result = parser.parse_args(&["--name", "John", "unexpected"]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Unexpected argument: unexpected");
    }

    #[test]
    fn test_parse_args_duplicate_argument() {
        let parser = create_basic_parser();
        let result = parser.parse_args(&["--name", "John", "--name", "Jane"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("name"), Some(&"Jane".to_string()));
    }

    #[test]
    fn test_subcommand_chain() {
        let mut main_parser = ArgumentParser::new("Main parser");
        let mut sub_parser1 = ArgumentParser::new("Sub parser 1");
        let sub_parser2 = ArgumentParser::new("Sub parser 2");
        sub_parser1.add_subcommand("sub2", sub_parser2);
        main_parser.add_subcommand("sub1", sub_parser1);

        let result = main_parser.parse_args(&["sub1", "sub2"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert!(namespace.subcommand.is_some());
        let (sub1_name, sub1_ns) = namespace.subcommand.unwrap();
        assert_eq!(sub1_name, "sub1");
        assert!(sub1_ns.subcommand.is_some());
        let (sub2_name, _) = sub1_ns.subcommand.unwrap();
        assert_eq!(sub2_name, "sub2");
    }

    #[test]
    fn test_parse_args_mixed_order() {
        let parser = create_basic_parser();
        let result = parser.parse_args(&["--age", "30", "--name", "John"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("name"), Some(&"John".to_string()));
        assert_eq!(namespace.values.get("age"), Some(&"30".to_string()));
    }

    #[test]
    fn test_parse_args_empty() {
        let parser = create_basic_parser();
        let result = parser.parse_args(&[]);
        assert!(result.is_err(), "{result:?}");
        assert_eq!(result.unwrap_err(), "Missing required argument: name");
    }

    #[test]
    fn test_parse_args_only_optional() {
        let mut parser = ArgumentParser::new("Test parser");
        parser.add_argument(
            "opt",
            Some('o'),
            ArgumentType::String,
            false,
            "Optional",
        );
        let result = parser.parse_args(&[]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert!(namespace.values.is_empty());
    }

    // This test might fail the implementation if it doesn't handle this edge case
    #[test]
    fn test_parse_args_boolean_with_value() {
        let mut parser = ArgumentParser::new("Test parser");
        parser.add_argument(
            "flag",
            Some('f'),
            ArgumentType::Boolean,
            false,
            "Flag",
        );
        let result = parser.parse_args(&["--flag", "true"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("flag"), Some(&"true".to_string()));
    }
}
