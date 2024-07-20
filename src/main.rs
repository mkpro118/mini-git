use mini_git::core::init;
use mini_git::utils::argparse::ArgumentParser;

fn main() {
    println!("Hello, world!");

    let exit_code = run();
    std::process::exit(exit_code);
}

fn run() -> i32 {
    let mut parser = make_parser();
    parser.compile();
    let Ok(args) = parser.parse_cli() else {
        unreachable!();
    };

    let Some((cmd, args)) = args.subcommand() else {
        unreachable!();
    };

    let res = match cmd.as_str() {
        "init" => init::cmd_init(args),
        _ => unreachable!(),
    };

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

    parser.add_subcommand("init", init::make_parser());

    parser
}
