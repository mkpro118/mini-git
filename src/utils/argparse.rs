//! # Argparse Module
//!
//! This module provides a flexible and easy-to-use command-line argument parsing library.
//! It supports various argument types, subcommands, and generates help messages automatically.
//!
//! ## Features
//!
//! - Support for different argument types (String, Integer, Float, Boolean)
//! - Short and long option formats
//! - Required and optional arguments
//! - Subcommand support
//! - Automatic help message generation
//!
//! ## Example
//!
//! ```
//! use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
//!
//! let mut parser = ArgumentParser::new("My CLI App");
//! parser.add_argument("name", ArgumentType::String)
//!     .required()
//!     .add_help("Your name");
//! parser.add_argument("age", ArgumentType::Integer)
//!     .short('a')
//!     .add_help("Your age");
//!
//! let args = parser.parse_cli().expect("Failed to parse arguments");
//! println!("Name: {}", args["name"]);
//! if let Some(age) = args.get("age") {
//!     println!("Age: {}", age);
//! }
//! ```

use std::collections::{HashMap, VecDeque};
use std::ops::Index;

/// Represents the type of an argument.
#[derive(Debug, Clone)]
pub enum ArgumentType {
    /// A string argument.
    String,
    /// An integer argument.
    Integer,
    /// A floating-point argument.
    Float,
    /// A boolean flag.
    Boolean,
}

/// Represents a single command-line argument.
#[derive(Debug)]
pub struct Argument {
    name: String,
    short: Option<char>,
    arg_type: ArgumentType,
    required: bool,
    help: String,
    default: Option<String>,
}

/// Represents a subcommand in the argument parser.
#[derive(Debug)]
struct SubCommand {
    name: String,
    parser: ArgumentParser,
}

/// The main argument parser struct.
#[derive(Debug)]
pub struct ArgumentParser {
    description: String,
    arguments: Vec<Argument>,
    subcommands: Vec<SubCommand>,
    cmd_chain: Option<String>,
    auto_exit: bool,
    exit_code: i32,
}

/// Represents the parsed arguments.
#[derive(Debug)]
pub struct Namespace {
    values: HashMap<String, String>,
    subcommand: Option<(String, Box<Namespace>)>,
}

impl Default for Argument {
    fn default() -> Self {
        Self {
            name: String::from("unknown"),
            short: None,
            arg_type: ArgumentType::String,
            required: false,
            help: String::from("No help provided"),
            default: None,
        }
    }
}

impl Argument {
    /// Creates a new `Argument` with the given name and type.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    ///
    /// let verbose = Argument::new("verbose", ArgumentType::Boolean);
    /// println!("{verbose:?}");
    /// ```
    #[must_use]
    pub fn new(name: &str, arg_type: ArgumentType) -> Self {
        Argument {
            name: name.to_string(),
            arg_type,
            ..Default::default()
        }
    }

    /// Sets the name of the argument.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    /// # fn foo(a: &Argument) {}
    /// # fn bar(a: &Argument) {}
    ///
    /// let mut arg_x = Argument::new("x", ArgumentType::Boolean);
    /// // Oh no, fn foo expects the arg to be called "foo",
    /// // And fn bar expects the arg to be called "bar",
    ///
    /// arg_x.name("foo");
    /// foo(&arg_x);
    ///
    /// arg_x.name("bar");
    /// bar(&arg_x);
    /// ```
    pub fn name(&mut self, name: &str) -> &mut Self {
        name.clone_into(&mut self.name);
        self
    }

    /// Sets the short option character for the argument.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    ///
    /// let mut verbose = Argument::new("verbose", ArgumentType::Boolean);
    /// verbose.short('v');
    ///
    /// // Now "-v" is accepted as a shorthand for "--verbose"
    /// ```
    pub fn short(&mut self, short: char) -> &mut Self {
        self.short = Some(short);
        self
    }

    /// Makes the argument required.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    ///
    /// let mut foo = Argument::new("foo", ArgumentType::String);
    /// foo.required();
    ///
    /// // ArgumentParser will fail if "--foo VALUE" is not provided
    /// ```
    pub fn required(&mut self) -> &mut Self {
        self.required = true;
        self
    }

    /// Makes the argument optional. This is the default when creating a new
    /// object
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    ///
    /// let mut foo = Argument::new("foo", ArgumentType::String);
    /// foo.optional();  // This is the default, so this does not really do
    ///                  // anything
    ///
    /// // ArgumentParser will not fail if "--foo VALUE" is not provided
    /// ```
    pub fn optional(&mut self) -> &mut Self {
        self.required = false;
        self
    }

    /// Sets the help message for the argument.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    ///
    /// let mut foo = Argument::new("foo", ArgumentType::String);
    /// foo.add_help("This is the foo argument");
    ///
    /// assert_eq!(foo.help(), "This is the foo argument");
    /// ```
    pub fn add_help(&mut self, help: &str) -> &mut Self {
        help.clone_into(&mut self.help);
        self
    }

    /// Returns the help message for the argument.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    ///
    /// let mut foo = Argument::new("foo", ArgumentType::String);
    /// foo.add_help("This is the foo argument");
    ///
    /// assert_eq!(foo.help(), "This is the foo argument");
    /// ```
    #[must_use]
    pub fn help(&self) -> &str {
        &self.help
    }

    /// Sets the default value for the argument.
    ///
    /// # Panics
    ///
    /// Panics if called on a required argument.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    ///
    /// let mut foo = Argument::new("foo", ArgumentType::String);
    /// foo.default("bar");
    ///
    /// // If "--foo VALUE" is not provided, foo will have the value "bar"
    /// ```
    pub fn default(&mut self, default: &str) -> &mut Self {
        assert!(
            !self.required,
            "default value cannot be set on a required argument"
        );
        self.default = Some(default.to_owned());
        self
    }
}

impl SubCommand {
    /// Creates a new `SubCommand` with the given name and parser.
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

impl Default for Namespace {
    fn default() -> Self {
        Self::new()
    }
}

impl Namespace {
    /// Creates a new empty `Namespace`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            subcommand: None,
        }
    }

    /// Sets a subcommand in the namespace.
    pub fn set_subcommand(&mut self, name: &str, namespace: Namespace) {
        self.subcommand = Some((name.to_owned(), Box::new(namespace)));
    }

    /// Gets the value of an argument by its name.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }

    /// Gets the subcommand, if any.
    #[must_use]
    pub fn subcommand(&self) -> Option<(&String, &Namespace)> {
        if let Some((ref cmd, ref namespace)) = &self.subcommand {
            Some((cmd, namespace))
        } else {
            None
        }
    }
}

impl Index<&str> for Namespace {
    type Output = String;

    /// Allows indexing into the `Namespace` to retrieve arguments
    fn index(&self, index: &str) -> &Self::Output {
        &self.values[index]
    }
}

impl Default for ArgumentParser {
    fn default() -> Self {
        Self {
            description: String::from("No description"),
            arguments: Vec::new(),
            subcommands: Vec::new(),
            cmd_chain: None,
            auto_exit: true,
            exit_code: 0,
        }
    }
}

impl ArgumentParser {
    /// Creates a new `ArgumentParser` with the given description.
    #[must_use]
    pub fn new(description: &str) -> Self {
        let mut parser = ArgumentParser {
            description: description.to_string(),
            ..Default::default()
        };
        parser
            .add_argument("help", ArgumentType::Boolean)
            .short('h')
            .optional()
            .add_help("Display this help message");
        parser
    }

    /// Whether or not to exit the program if there are errors in
    /// parsing the arguments.
    ///
    /// This is only relevant if [`ArgumentParser::parse_cli`] is used.
    /// The exit code can be set using [`ArgumentParser::exit_code`], defaults to 0.
    pub fn auto_exit(&mut self, auto_exit: bool) -> &mut Self {
        auto_exit.clone_into(&mut self.auto_exit);
        self
    }

    /// Sets the exit code
    pub fn exit_code(&mut self, exit_code: i32) -> &mut Self {
        exit_code.clone_into(&mut self.exit_code);
        self
    }

    /// Adds a new argument to the parser, and returns a mutable reference
    /// to the added [`Argument`]
    #[allow(clippy::missing_panics_doc)]
    pub fn add_argument(
        &mut self,
        name: &str,
        arg_type: ArgumentType,
    ) -> &mut Argument {
        self.arguments.push(Argument::new(name, arg_type));
        self.arguments.last_mut().unwrap()
    }

    /// Adds a subcommand to the parser.
    pub fn add_subcommand(&mut self, name: &str, parser: ArgumentParser) {
        self.subcommands.push(SubCommand::new(name, parser));
    }

    /// Parses command-line arguments.
    ///
    /// This is essentially a wrapper over [`ArgumentParser::parse_args`], where the arguments
    /// are obtained from [`std::env::args`].
    ///
    /// # Errors
    ///
    /// This function may fail if,
    /// - Not all required arguments were found.
    /// - Non-boolean arguments are missing values.
    ///
    /// This function will automatically exit the program unless auto exit is
    /// disabled using [`ArgumentParser::auto_exit`].
    ///
    /// If auto exit is disabled, a [`String`] describing the error is returned.
    ///
    /// The default exit code is 0, but can be set using
    /// [`ArgumentParser::exit_code`]
    pub fn parse_cli(&self) -> Result<Namespace, String> {
        let args = std::env::args().skip(1);
        match self.parse(args, true) {
            Ok(res) => Ok(res),
            Err(msg) if self.auto_exit => {
                println!("{msg}");
                std::process::exit(0);
            }
            Err(msg) => Err(msg),
        }
    }

    /// Parses the given array of argument strings.
    ///
    /// # Errors
    ///
    /// This function may fail if,
    /// - Not all required arguments were found.
    /// - Non-boolean arguments are missing values.
    ///
    /// A [`String`] describing the error is returned.
    pub fn parse_args(&self, args: &[&str]) -> Result<Namespace, String> {
        self.parse(args.iter().map(|&x| x.to_owned()), false)
    }

    fn parse<I>(&self, mut args: I, cli: bool) -> Result<Namespace, String>
    where
        I: Iterator<Item = String>,
    {
        let mut positionals = self
            .arguments
            .iter()
            .filter(|a| a.required)
            .collect::<VecDeque<&Argument>>();
        dbg!(&positionals);

        let mut parsed = Namespace::new();

        loop {
            let Some(arg) = args.next() else {
                break;
            };

            dbg!(&arg);

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
                let (find_strategy, err) = if let Some(name) =
                    arg.strip_prefix("--")
                {
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
                    positionals.retain(|a| a.name != argument.name);
                } else {
                    return Err(format!("Unknown argument: {arg}"));
                }
            } else {
                // Positional argument
                if positionals.is_empty() {
                    return Err(format!(
                        "Unexpected positional argument: {arg}"
                    ));
                }

                println!("in loop");
                dbg!(&positionals);

                if let Some(argument) = self
                    .arguments
                    .iter()
                    .find(|a| a.name == positionals[0].name)
                {
                    parsed.values.insert(argument.name.clone(), arg.clone());
                } else {
                    return Err(format!("Unexpected argument: {arg}"));
                }

                positionals.pop_front();
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

    /// Generates a help message for the parser.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn help(&self) -> String {
        let name = std::env::args().next().expect("executable path");
        let name = std::path::Path::new(&name)
            .file_stem()
            .expect("executable name")
            .to_str()
            .expect("valid utf-8");
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
                .map_or_else(|| " ".repeat(4), |c| format!("-{c}, "));
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
        parser
            .add_argument("name", ArgumentType::String)
            .short('n')
            .required()
            .add_help("Name");
        parser
            .add_argument("age", ArgumentType::Integer)
            .short('a')
            .add_help("Age");
        parser
    }

    #[test]
    fn test_argument_creation() {
        let mut arg = Argument::new("test", ArgumentType::String);
        arg.short('t').required().add_help("Test arg");

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
        parser
            .add_argument("test", ArgumentType::String)
            .short('t')
            .required()
            .add_help("Test arg");

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
        parser
            .add_argument("flag", ArgumentType::Boolean)
            .short('f')
            .add_help("Flag");
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
        sub_parser
            .add_argument("sub_arg", ArgumentType::String)
            .short('s')
            .required()
            .add_help("Sub arg");

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
        assert!(result
            .is_err_and(|msg| msg.contains("Unexpected positional argument")));
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
        parser
            .add_argument("opt", ArgumentType::String)
            .short('o')
            .add_help("Optional");
        let result = parser.parse_args(&[]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert!(namespace.values.is_empty());
    }

    // This test might fail the implementation if it doesn't handle this edge case
    #[test]
    fn test_parse_args_boolean_with_value() {
        let mut parser = ArgumentParser::new("Test parser");
        parser
            .add_argument("flag", ArgumentType::Boolean)
            .short('f')
            .optional()
            .add_help("Flag");

        let result = parser.parse_args(&["--flag"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("flag"), Some(&"true".to_string()));
    }
}
