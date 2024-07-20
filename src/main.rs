use mini_git::core::{hash_object, init};
use mini_git::utils::argparse::{ArgumentParser, Namespace};

struct Command {
    name: &'static str,
    make_parser: fn() -> ArgumentParser,
    callback: fn(&Namespace) -> Result<String, String>,
}

impl Command {
    pub const fn new(
        name: &'static str,
        make_parser: fn() -> ArgumentParser,
        callback: fn(&Namespace) -> Result<String, String>,
    ) -> Self {
        Self {
            name,
            make_parser,
            callback,
        }
    }
}

static COMMAND_MAP: &[Command] = &[
    Command::new("init", init::make_parser, init::cmd_init),
    Command::new(
        "hash-object",
        hash_object::make_parser,
        hash_object::cmd_hash_object,
    ),
];

fn main() {
    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    let mut parser = make_parser();
    parser.compile();
    let Ok(args) = parser.parse_cli() else {
        unreachable!();
    };

    let Some((command, args)) = args.subcommand() else {
        unreachable!();
    };

    let res = COMMAND_MAP
        .binary_search_by(|cmd| cmd.name.cmp(&command))
        .map(|x| (COMMAND_MAP[x].callback)(args))
        .expect("Should not be an invalid command");

    match res {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(msg) => {
            println!("{msg}");
            -1
        }
    }
}

fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new("MiniGit, a git, but mini!");

    for command in COMMAND_MAP {
        parser.add_subcommand(command.name, (command.make_parser)())
    }

    parser
}
