use mini_git::core::commands::{
    cat_file, diff, hash_object, init, log, ls_files, ls_tree, rev_parse,
    show_ref,
};
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

macro_rules! cmd {
    ($name:literal, $cmd:ident) => {
        Command::new($name, $cmd::make_parser, $cmd::$cmd)
    };
}

// Needs to be in sorted order by name
const COMMAND_MAP: &[Command] = &[
    cmd!("cat-file", cat_file),
    cmd!("diff", diff),
    cmd!("hash-object", hash_object),
    cmd!("init", init),
    cmd!("log", log),
    cmd!("ls-files", ls_files),
    cmd!("ls-tree", ls_tree),
    cmd!("rev-parse", rev_parse),
    cmd!("show-ref", show_ref),
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
        .binary_search_by(|cmd| cmd.name.cmp(command))
        .map(|x| (COMMAND_MAP[x].callback)(args))
        .expect("Should not be an invalid command");

    match res {
        Ok(msg) => {
            if msg.ends_with('\n') {
                print!("{msg}");
            } else {
                println!("{msg}");
            }
            0
        }
        Err(msg) => {
            if msg.ends_with('\n') {
                print!("{msg}");
            } else {
                println!("{msg}");
            }
            -1
        }
    }
}

fn make_parser() -> ArgumentParser {
    let mut parser = ArgumentParser::new("MiniGit, a git, but mini!");

    for command in COMMAND_MAP {
        parser.add_subcommand(command.name, (command.make_parser)());
    }

    parser.require_subcommand();

    parser
}

// The following code ensures that the Command array is sorted at compile time.
// The Command array is required to be sorted to be binary-search friendly,
// and we enforce this at compile time.
#[allow(dead_code)]
const fn str_le(a: &'static str, b: &'static str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut i = 0;
    let len = if a.len() < b.len() { a.len() } else { b.len() };

    while i < len {
        if a[i] < b[i] {
            return true;
        } else if a[i] > b[i] {
            return false;
        }
        i += 1;
    }
    len == a.len()
}

#[allow(dead_code)]
const fn is_cmd_sorted() -> bool {
    let len = COMMAND_MAP.len();
    assert!(len > 1, "COMMAND MAP IS EMPTY");
    let mut prev_name = &COMMAND_MAP[0].name;
    let mut i = 1;

    while i < len {
        if !str_le(prev_name, COMMAND_MAP[i].name) {
            return false;
        }

        prev_name = &COMMAND_MAP[i].name;
        i += 1;
    }

    true
}

// If this fails to compile, the command array is not sorted
#[allow(clippy::erasing_op)]
const _: u8 = 0 / is_cmd_sorted() as u8;
