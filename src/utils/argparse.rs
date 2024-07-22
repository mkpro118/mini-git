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
//! ```no_run
//! use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
//!
//! let mut parser = ArgumentParser::new("My CLI App");
//! parser.add_argument("name", ArgumentType::String)
//!     .required()
//!     .add_help("Your name");
//! parser.add_argument("age", ArgumentType::Integer)
//!     .short('a')
//!     .add_help("Your age");
//! parser.compile();
//!
//! let args = parser.parse_cli().expect("Failed to parse arguments");
//! println!("Name: {}", args["name"]);
//! if let Some(age) = args.get("age") {
//!     println!("Age: {}", age);
//! }
//! ```

use std::collections::{HashMap, HashSet, VecDeque};
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
    choices: Option<HashSet<String>>,
    ignore_case: bool,
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
    compiled: bool,
    subcommand_required: bool,
    max_arg_len: usize,
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
            choices: None,
            ignore_case: false,
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

    /// Sets the choices for a given argument. A parser will fail if the value
    /// provided is not one of the given choices.
    ///
    /// By default, the choices are case sensitive. Use the
    /// [`ArgumentParser::ignore_case`] method to allow case insensitive choices
    ///
    /// # Panics
    ///
    /// If called on an argument that has type [`ArgumentType::Boolean`]
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    ///
    /// let mut operation = Argument::new("operation", ArgumentType::String);
    /// operation.choices(&["add", "subtract", "multiply", "divide"]);
    /// ```
    pub fn choices(&mut self, choices: &[&str]) -> &mut Self {
        assert!(
            !matches!(self.arg_type, ArgumentType::Boolean),
            "Choices cannot be used with boolean arguments"
        );
        self.choices = Some(
            choices
                .iter()
                .copied()
                .map(String::from)
                .collect::<HashSet<String>>(),
        );
        self
    }

    /// Accept case insensitive values for choices.
    ///
    /// By default, the choices are case sensitive. This method to allows
    /// accepting arguments that match choices ignoring case.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{Argument, ArgumentType};
    ///
    /// let mut operation = Argument::new("operation", ArgumentType::String);
    /// operation
    ///     .choices(&["add", "subtract", "multiply", "divide"])
    ///     .ignore_case();
    ///
    /// // Now "add", "Add", "ADD", "aDD" are all accepted for `operation`.
    /// ```
    pub fn ignore_case(&mut self) -> &mut Self {
        self.ignore_case = true;
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
        self.default = Some(default.to_owned());
        self
    }
}

impl SubCommand {
    /// Creates a new `SubCommand` with the given name and parser.
    ///
    /// Note that this function takes ownership of the parser, as subcommands
    /// own their parsers
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
            compiled: false,
            subcommand_required: false,
            max_arg_len: 0,
        }
    }
}

impl ArgumentParser {
    /// Creates a new `ArgumentParser` with the given description.
    ///
    /// This method automatically adds a `--help` flag to the parser.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
    ///
    /// let parser = ArgumentParser::new("My CLI Application");
    /// ```
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

    /// Sets whether the program should automatically exit if there are errors in
    /// parsing the arguments.
    ///
    /// This is only relevant if [`ArgumentParser::parse_cli`] is used.
    /// The exit code can be set using [`ArgumentParser::exit_code`], defaults to 0.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::ArgumentParser;
    ///
    /// let mut parser = ArgumentParser::new("My CLI Application");
    /// parser.auto_exit(false);
    /// ```
    pub fn auto_exit(&mut self, auto_exit: bool) -> &mut Self {
        auto_exit.clone_into(&mut self.auto_exit);
        self
    }

    /// Sets the exit code to be used when `auto_exit` is true.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::ArgumentParser;
    ///
    /// let mut parser = ArgumentParser::new("My CLI Application");
    /// parser.auto_exit(true).exit_code(1);
    /// ```
    pub fn exit_code(&mut self, exit_code: i32) -> &mut Self {
        exit_code.clone_into(&mut self.exit_code);
        self
    }

    /// Adds a new argument to the parser, and returns a mutable reference
    /// to the added [`Argument`].
    ///
    /// # Panics
    ///
    /// Panics if an argument with the same name is added twice.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
    ///
    /// let mut parser = ArgumentParser::new("My CLI Application");
    /// parser.add_argument("name", ArgumentType::String)
    ///     .required()
    ///     .add_help("Your name");
    /// parser.add_argument("age", ArgumentType::Integer)
    ///     .optional()
    ///     .add_help("Your age");
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn add_argument(
        &mut self,
        name: &str,
        arg_type: ArgumentType,
    ) -> &mut Argument {
        if let Some(arg) = self.arguments.iter().find(|a| a.name == name) {
            panic!(
                "Argument \"{name}\" already exists. (type = {:?}, help = {})",
                arg.arg_type,
                arg.help()
            );
        }
        self.compiled = false;
        self.arguments.push(Argument::new(name, arg_type));
        self.max_arg_len = self.max_arg_len.max(name.len());
        self.arguments.last_mut().unwrap()
    }

    /// Adds a subcommand to the parser.
    ///
    /// # Panics
    ///
    /// Panics if a subcommand with the same name is added twice.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
    ///
    /// let mut main_parser = ArgumentParser::new("Main Application");
    /// let mut sub_parser = ArgumentParser::new("Subcommand");
    /// sub_parser.add_argument("sub_arg", ArgumentType::String)
    ///     .required()
    ///     .add_help("Subcommand argument");
    ///
    /// main_parser.add_subcommand("sub", sub_parser);
    /// ```
    pub fn add_subcommand(
        &mut self,
        name: &str,
        parser: ArgumentParser,
    ) -> &mut Self {
        assert!(
            !self.subcommands.iter().any(|c| c.name == name),
            "Subcommand \"{name}\" already exists."
        );
        self.compiled = false;
        self.subcommands.push(SubCommand::new(name, parser));
        self
    }

    /// Requires at least one subcommand to be parsed.
    ///
    /// ```
    /// use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
    ///
    /// let mut main_parser = ArgumentParser::new("Main Application");
    /// let mut sub_parser1 = ArgumentParser::new("Subcommand1");
    /// sub_parser1.add_argument("sub_arg", ArgumentType::String)
    ///     .required()
    ///     .add_help("Subcommand argument");
    ///
    /// let mut sub_parser2 = ArgumentParser::new("Subcommand2");
    /// sub_parser2.add_argument("sub_arg", ArgumentType::String)
    ///     .required()
    ///     .add_help("Subcommand argument");
    ///
    /// main_parser.add_subcommand("sub1", sub_parser1);
    /// main_parser.add_subcommand("sub2", sub_parser2);
    /// main_parser.require_subcommand();
    /// ```
    pub fn require_subcommand(&mut self) -> &mut Self {
        self.subcommand_required = true;
        self
    }

    #[must_use]
    pub fn closest_subcommands(
        &self,
        to: &str,
        max_dist: usize,
        count: usize,
    ) -> Vec<String> {
        self.subcommands
            .iter()
            .map(|cmd| cmd.name.clone())
            .filter(|name| dl_distance(name, to) <= max_dist)
            .take(count)
            .collect()
    }

    /// Compiles the argument parser, checking for any conflicts in the
    /// argument definitions.
    ///
    /// This method should be called after all arguments and subcommands have
    /// been added, but before parsing any arguments. It checks for conflicts
    /// such as duplicate short options.
    ///
    /// This function will recursively compile all subcommand parsers.
    ///
    /// # Panics
    ///
    /// Panics if there are any conflicts in the argument definitions, such as:
    /// - Two arguments with the same short option
    ///
    /// # Example
    ///
    /// ```should_panic
    /// use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
    ///
    /// let mut parser = ArgumentParser::new("My CLI Application");
    /// parser.add_argument("name", ArgumentType::String)
    ///     .required()
    ///     .short('n')
    ///     .add_help("Your name");
    ///
    /// // Accidentally adding an argument with the same name
    /// parser.add_argument("name", ArgumentType::Integer)
    ///     .optional()
    ///     .short('a')
    ///     .add_help("Your age");
    ///
    /// parser.compile(); // Will panic!
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn compile(&mut self) {
        if self.compiled {
            return;
        }
        let Err((short, arg1, arg2)) = self
            .arguments
            .iter()
            .filter(|a| a.short.is_some())
            .try_fold(
                std::collections::HashMap::<char, &str>::new(),
                |mut map, arg| {
                    let short = arg.short.as_ref().unwrap();
                    if map.contains_key(short) {
                        Err((*short, map[short], arg.name.clone()))
                    } else {
                        map.insert(*short, &arg.name);
                        Ok(map)
                    }
                },
            )
        else {
            for subcommand in &mut self.subcommands {
                subcommand.parser.compile();
            }
            self.compiled = true;
            return;
        };

        panic!(
            "Found two arguments \"{arg1}\" and \"{arg2}\" \
                with the same shorthand '-{short}' in parser {}",
            self.description
        );
    }

    fn required_positionals(&self) -> VecDeque<&Argument> {
        self.arguments.iter().filter(|a| a.required).collect()
    }

    /// Parses command-line arguments.
    ///
    /// This is essentially a wrapper over [`ArgumentParser::parse_args`], where the arguments
    /// are obtained from [`std::env::args`].
    ///
    /// # Errors
    ///
    /// This function may fail if:
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
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
    ///
    /// let mut parser = ArgumentParser::new("My CLI Application");
    /// parser.add_argument("name", ArgumentType::String)
    ///     .required()
    ///     .add_help("Your name");
    ///
    /// parser.compile();
    /// let args = parser.parse_cli().expect("Failed to parse arguments");
    /// println!("Hello, {}!", args["name"]);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
    ///
    /// let mut parser = ArgumentParser::new("My CLI Application");
    /// parser.add_argument("name", ArgumentType::String)
    ///     .required()
    ///     .add_help("Your name");
    /// parser.add_argument("age", ArgumentType::Integer)
    ///     .optional()
    ///     .add_help("Your age");
    ///
    /// parser.compile();
    /// let args = parser.parse_args(&["--name", "Alice", "--age", "30"])
    ///     .expect("Failed to parse arguments");
    /// println!("Name: {}", args["name"]);
    /// if let Some(age) = args.get("age") {
    ///     println!("Age: {}", age);
    /// }
    /// ```
    pub fn parse_args(&self, args: &[&str]) -> Result<Namespace, String> {
        self.parse(args.iter().map(|&x| x.to_owned()), false)
    }

    fn parse<I>(&self, mut args: I, cli: bool) -> Result<Namespace, String>
    where
        I: Iterator<Item = String>,
    {
        assert!(
            self.compiled,
            "parser has not been compiled!\n  Help: use parser.compile() \
            before using parse_{}",
            if cli { "cli" } else { "args" }
        );

        let mut parsed = Namespace::new();
        let mut first_positional = None;
        let mut positionals = self.required_positionals();

        while let Some(arg) = args.next() {
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
                if (self.handle_optional(
                    &mut parsed,
                    &arg,
                    &mut args,
                    &mut positionals,
                    cli,
                )?)
                .is_some()
                {
                    return Ok(parsed);
                };
            } else {
                self.handle_positional(
                    &mut parsed,
                    &arg,
                    &mut positionals,
                    &mut first_positional,
                )?;
            }
        }

        self.check_subcommand(&parsed, first_positional)?;
        self.check_required(&mut parsed)?;

        Ok(parsed)
    }

    fn handle_optional<'a, 'b, I>(
        &'a self,
        parsed: &'b mut Namespace,
        arg: &String,
        args: &mut I,
        positionals: &mut VecDeque<&Argument>,
        cli: bool,
    ) -> Result<Option<&'b mut Namespace>, String>
    where
        I: Iterator<Item = String>,
        'a: 'b,
    {
        let (find_strategy, err) = if let Some(name) = arg.strip_prefix("--") {
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

        if let Some(argument) = self.arguments.iter().find(find_strategy) {
            if argument.name == "help" {
                if cli {
                    println!("{}", self.help());
                    if self.auto_exit {
                        std::process::exit(self.exit_code);
                    }
                } else {
                    parsed.values.clear();
                    Self::insert_argument(parsed, argument, arg.clone())?;
                }
                return Ok(Some(parsed));
            }

            if matches!(argument.arg_type, ArgumentType::Boolean) {
                parsed
                    .values
                    .insert(argument.name.clone(), "true".to_string());
            } else {
                let Some(val) = args.next() else {
                    return err;
                };
                Self::insert_argument(parsed, argument, val)?;
            }
            positionals.retain(|a| a.name != argument.name);
        } else {
            return Err(format!("Unknown argument: {arg}"));
        }

        Ok(None)
    }

    fn handle_positional(
        &self,
        parsed: &mut Namespace,
        arg: &String,
        positionals: &mut VecDeque<&Argument>,
        first_positional: &mut Option<String>,
    ) -> Result<(), String> {
        // Positional argument
        if positionals.is_empty() {
            if self.subcommand_required && parsed.subcommand.is_none() {
                return self.check_subcommand(parsed, Some(arg.clone()));
            }
            return Err(format!("Unexpected positional argument: {arg}"));
        }

        if let Some(argument) = self
            .arguments
            .iter()
            .find(|a| a.name == positionals[0].name)
        {
            if first_positional.is_none() {
                *first_positional = Some(arg.clone());
            }
            Self::insert_argument(parsed, argument, arg.to_string())?;
        } else {
            return Err(format!("Unexpected argument: {arg}"));
        }

        positionals.pop_front();
        Ok(())
    }

    fn insert_argument(
        parsed: &mut Namespace,
        argument: &Argument,
        value: String,
    ) -> Result<(), String> {
        if let Some(ref options) = argument.choices {
            let compare_strategy = if argument.ignore_case {
                let val = value.to_lowercase();
                Box::new(move |x: &String| x.to_lowercase() == val)
                    as Box<dyn FnMut(&String) -> bool>
            } else {
                Box::new(|x: &String| *x == value)
                    as Box<dyn FnMut(&String) -> bool>
            };

            if !options.iter().any(compare_strategy) {
                return Err(format!("not a choice: {value}"));
            }
        }

        match argument.arg_type {
            ArgumentType::Integer => {
                if value.parse::<isize>().is_err() {
                    return Err(format!(
                        "Expected integer value for '{}', \
                    found {value}",
                        argument.name,
                    ));
                }
            }
            ArgumentType::Float => {
                if value.parse::<f64>().is_err() {
                    return Err(format!(
                        "Expected float value for '{}', \
                    found {value}",
                        argument.name,
                    ));
                }
            }
            ArgumentType::Boolean if argument.name != "help" => unreachable!(),
            _ => {}
        };

        parsed.values.insert(argument.name.clone(), value);
        Ok(())
    }

    fn check_subcommand(
        &self,
        parsed: &Namespace,
        first: Option<String>,
    ) -> Result<(), String> {
        if parsed.subcommand.is_some() || !self.subcommand_required {
            return Ok(());
        }

        let Some(first) = first else {
            return Err(self.help());
        };

        let name = Self::exec_name();
        let mut help = format!("\"{first}\" is not a {name} command.");
        let matches = self.closest_subcommands(&first, 3, 3);

        if matches.is_empty() {
            return Err(format!("{help} See '{name} --help'"));
        }

        help.push_str("\n\nSimilar subcommands are:\n");

        for sub in matches {
            help.push_str("  ");
            help.push_str(&sub);
            help.push('\n');
        }

        Err(help)
    }

    // Check all required arguments are provided, and set defaults otherwise
    fn check_required(&self, parsed: &mut Namespace) -> Result<(), String> {
        for arg in &self.arguments {
            // If not already found
            if !parsed.values.contains_key(&arg.name) {
                // If has default, use default
                if let Some(default) = &arg.default {
                    parsed.values.insert(arg.name.clone(), default.clone());
                    continue;
                }

                // If has no default, but it required.
                if arg.required {
                    return Err(format!(
                        "Missing required argument: {}",
                        arg.name
                    ));
                }
            }
        }
        Ok(())
    }

    /// Generates a help message for the parser.
    ///
    /// This method is automatically called when the `--help` flag is used.
    ///
    /// # Example
    ///
    /// ```
    /// use mini_git::utils::argparse::{ArgumentParser, ArgumentType};
    ///
    /// let mut parser = ArgumentParser::new("My CLI Application");
    /// parser.add_argument("name", ArgumentType::String)
    ///     .required()
    ///     .add_help("Your name");
    /// parser.add_argument("age", ArgumentType::Integer)
    ///     .optional()
    ///     .add_help("Your age");
    ///
    /// println!("{}", parser.help());
    /// ```
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn help(&self) -> String {
        let name = Self::exec_name();

        // First line, usage text
        let mut help_text = format!(
            "Usage: {name} {} [options]",
            self.cmd_chain.as_ref().map_or("", |x| x.as_str())
        );

        // First line, usage text positional args
        for positional in self.required_positionals() {
            help_text.push(' ');
            // If positional arg has default, display it as an optional arg
            if positional.default.is_some() {
                help_text.push_str("[ --");
                help_text.push_str(&positional.name);
                help_text.push(' ');
                help_text.push_str(&positional.name.to_uppercase());
                help_text.push_str(" ]");
            } else {
                help_text.push_str(&positional.name.to_uppercase());
            }
        }

        // First line, usage text, subcommands if any
        if !self.subcommands.is_empty() {
            help_text.push_str(" [SUBCOMMAND]");
        }

        help_text.push_str("\n\n");

        // Next line, descriptoin
        help_text.push_str(&self.description);

        // Next line, options header
        help_text.push_str("\n\nOptions:\n");

        // List all options
        for arg in &self.arguments {
            let has_default = arg.default.is_some();
            let short = arg
                .short
                .map_or_else(|| " ".repeat(4), |c| format!("-{c}, "));

            let required = if arg.required && !has_default {
                " (required)"
            } else {
                ""
            };

            // Spaces to ensure all help text starts on the same column
            let padding = " ".repeat(self.max_arg_len - arg.name.len() + 4);

            // {short} {name} {padding} {help} {required}
            help_text.push_str(&format!(
                "  {short}--{}{padding} {} {required}\n",
                arg.name, arg.help
            ));

            // For options that have choices, list the choices on the next line
            if let Some(ref choices) = arg.choices {
                let indent = 2 + 4 + 2 + self.max_arg_len + 1 + 4 + 2;
                help_text.push_str(&" ".repeat(indent));
                help_text.push_str("Choices: [ ");

                let mut choices =
                    choices.iter().map(String::as_str).collect::<Vec<&str>>();

                // arg.choices is a set, sort to ensure consistent help message
                choices.sort_unstable();

                let choices = choices.join(", ");
                help_text.push_str(&choices);

                if arg.ignore_case {
                    help_text.push_str(" (case insensitive)");
                }
                help_text.push_str(" ]\n");
            }
        }

        // List all subcommands and their descriptions
        if !self.subcommands.is_empty() {
            help_text.push_str("\nSubcommands:\n");
            for subcommand in &self.subcommands {
                help_text.push_str(&format!(
                    "  {:<16} {}\n",
                    subcommand.name, subcommand.parser.description
                ));
            }
        }

        help_text
    }

    fn exec_name() -> String {
        let name = std::env::args().next().expect("executable path");
        std::path::Path::new(&name)
            .file_stem()
            .expect("executable name")
            .to_str()
            .expect("valid utf-8")
            .to_owned()
    }
}

/// Damerau–Levenshtein distance with adjacent transpositions
/// This function is case sensitive
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn dl_distance(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (len_a, len_b) = (a.len(), b.len());

    let max_dist = len_a + len_b;

    let mut da = [0; 128];
    let mut dist = vec![vec![0; len_b + 2]; len_a + 2];

    let idx = |x: i32| (x + 1) as usize;

    macro_rules! dist {
        ($idx1:expr, $idx2:expr) => {
            dist[idx(($idx1) as i32)][idx(($idx2) as i32)]
        };
    }

    dist!(-1, -1) = max_dist;

    for i in 0..=len_a {
        dist!(i, -1) = max_dist;
        dist!(i, 0) = i;
    }

    for i in 0..=len_b {
        dist!(-1, i) = max_dist;
        dist!(0, i) = i;
    }

    for i in 1..=len_a {
        let mut db = 0;

        for j in 1..=len_b {
            let k = da[b[j - 1] as usize] as i32;
            let l = db as i32;
            let cost = if a[i - 1] == b[j - 1] {
                db = j;
                0
            } else {
                1
            };

            dist!(i, j) = {
                let substitution = dist!(i - 1, j - 1) + cost;
                let insertion = dist!(i, j - 1) + 1;
                let deletion = dist!(i - 1, j) + 1;
                let transposition =
                    dist!(k - 1, l - 1) + (i + j - 1) - ((k + l) as usize);
                substitution.min(insertion).min(deletion).min(transposition)
            };
        }

        da[a[i - 1] as usize] = i;
    }

    dist[idx(len_a as i32)][idx(len_b as i32)]
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
        parser.compile();
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
        parser.compile();
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
        parser.compile();

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
        main_parser.compile();

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
        parser.compile();
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
        parser.compile();

        let result = parser.parse_args(&["--flag"]);
        assert!(result.is_ok());
        let namespace = result.unwrap();
        assert_eq!(namespace.values.get("flag"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_args_with_choices() {
        let choices = ["add", "subtract", "multiply", "divide"];
        let mut parser = ArgumentParser::new("Test parser");
        parser
            .add_argument("ops", ArgumentType::String)
            .required()
            .choices(&choices);
        parser.compile();

        for op in choices {
            let result = parser.parse_args(&[op]);
            assert!(result.is_ok());
            let namespace = result.unwrap();
            assert_eq!(namespace.values.get("ops"), Some(&op.to_owned()));
        }
    }

    #[test]
    fn test_parse_args_with_bad_choices() {
        let choices = ["add", "subtract", "multiply", "divide"];
        let mut parser = ArgumentParser::new("Test parser");
        parser
            .add_argument("ops", ArgumentType::String)
            .required()
            .choices(&choices);
        parser.compile();

        let bad_choices = ["exp", "log", "pow"];
        for op in bad_choices {
            let result = parser.parse_args(&[op]);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_args_with_choices_ignore_case() {
        let choices = ["add", "subtract", "multiply", "divide"];
        let mut parser = ArgumentParser::new("Test parser");
        parser
            .add_argument("ops", ArgumentType::String)
            .required()
            .choices(&choices)
            .ignore_case();
        parser.compile();

        let choices = [
            "Add", "add", "ADD", "aDD", "Subtract", "subtract", "SUBTRACT",
            "suBTRAct", "Multiply", "subtract", "MULTIPLY", "muLTIply",
            "Divide", "divide", "DIVIDE", "diVIDe",
        ];

        for op in choices {
            let result = parser.parse_args(&[op]);
            assert!(result.is_ok());
            let namespace = result.unwrap();
            assert_eq!(namespace.values.get("ops"), Some(&op.to_owned()));
        }
    }

    fn setup_subcommand_parser() -> ArgumentParser {
        let mut main_parser = ArgumentParser::new("Main Application");
        main_parser.add_argument("arg1", ArgumentType::String);
        main_parser.add_argument("arg2", ArgumentType::String);

        let mut sub_parser1 = ArgumentParser::new("Subcommand1");
        sub_parser1
            .add_argument("sub_arg", ArgumentType::String)
            .required()
            .add_help("Subcommand argument");

        let mut sub_parser2 = ArgumentParser::new("Subcommand2");
        sub_parser2
            .add_argument("sub_arg", ArgumentType::String)
            .required()
            .add_help("Subcommand argument");

        main_parser
            .add_subcommand("sub1", sub_parser1)
            .add_subcommand("sub2", sub_parser2)
            .require_subcommand()
            .compile();
        main_parser
    }

    #[test]
    fn test_parse_args_required_subcommand_good() {
        let parser = setup_subcommand_parser();

        let good_args = [["sub1", "arg"], ["sub2", "arg"]];
        for args in good_args {
            let res = parser.parse_args(&args);
            assert!(res.is_ok());
            let res = res.unwrap();
            assert!(res.subcommand.is_some());
            let (cmd, namespace) = res.subcommand.unwrap();
            assert_eq!(cmd, args[0]);
            assert_eq!(namespace["sub_arg"], args[1]);
        }
    }

    #[test]
    fn test_parse_args_required_subcommand_bad() {
        let parser = setup_subcommand_parser();

        let bad_args = [["hello", "world"], ["foo", "bar"]];

        for args in bad_args {
            let res = parser.parse_args(&args);
            assert!(res.is_err());
        }
    }

    fn make_type_parser(float: bool) -> ArgumentParser {
        use ArgumentType::{Float, Integer};
        let mut parser = ArgumentParser::new("type_check");
        parser
            .add_argument("num1", if float { Float } else { Integer })
            .optional();
        parser
            .add_argument("num2", if float { Float } else { Integer })
            .optional();
        parser
            .add_argument("num3", if float { Float } else { Integer })
            .optional();

        parser.compile();
        parser
    }

    #[test]
    fn test_parse_args_type_check_integer_good() {
        let parser = make_type_parser(false);
        let good_args: [&[&str]; 3] =
            [&["--num1", "2"], &["--num2", "-3"], &["--num3", "123456"]];

        for args in good_args {
            let res = parser.parse_args(&args);
            assert!(res.is_ok());
            let res = res.unwrap();
            let key = args[0].strip_prefix("--").unwrap();
            assert_eq!(res[key], args[1]);
        }
    }

    #[test]
    fn test_parse_args_type_check_integer_bad() {
        let parser = make_type_parser(false);

        let bad_args: [&[&str]; 3] = [
            &["--num1", "a2"],
            &["--num2", "5-3"],
            &["--num3", "num123456"],
        ];

        for args in bad_args {
            let res = parser.parse_args(&args);
            assert!(res.is_err());
        }
    }

    #[test]
    fn test_parse_args_type_check_float_good() {
        let parser = make_type_parser(true);

        let good_args: [&[&str]; 3] = [
            &["--num1", "2.71"],
            &["--num2", "-3.141"],
            &["--num3", "123.456"],
        ];

        for args in good_args {
            let res = parser.parse_args(&args);
            assert!(res.is_ok());
            let res = res.unwrap();
            let key = args[0].strip_prefix("--").unwrap();
            assert_eq!(res[key], args[1]);
        }
    }

    #[test]
    fn test_parse_args_type_check_float_bad() {
        let parser = make_type_parser(true);

        let bad_args: [&[&str]; 3] = [
            &["--num1", "a2.0"],
            &["--num2", "53..00"],
            &["--num3", "num123.456"],
        ];

        for args in bad_args {
            let res = parser.parse_args(&args);
            assert!(res.is_err());
        }
    }

    #[test]
    fn test_dl_distance() {
        let data = [
            ("ca", "abc", 2),
            ("foo", "bar", 3),
            ("arg", "parse", 3),
            ("hello", "world", 4),
            ("damerau", "levenshtein", 10),
        ];

        for (x, y, dist) in data {
            assert_eq!(dl_distance(x, y), dist);
        }
    }
}
