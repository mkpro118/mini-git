#![allow(dead_code)]

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
