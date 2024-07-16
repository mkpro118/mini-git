use core::iter::Iterator;
use std::env;

use mini_git::core::init;

fn main() {
    println!("Hello, world!");

    let exit_code = run(env::args().skip(1));
    std::process::exit(exit_code);
}

fn run(mut args: impl Iterator<Item = impl AsRef<str>>) -> i32 {
    let Some(cmd) = args.next() else {
        println!("Help!");
        return 0;
    };

    let res = match cmd.as_ref() {
        "init" => init::cmd_init(args),
        _ => Ok("Help".to_string()),
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
