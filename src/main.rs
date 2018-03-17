#![allow(dead_code, unused_imports)]

#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
extern crate regex;

use std::result;
use std::process;

mod app;
mod blue;
mod capture;

type Result<T> = result::Result<T, failure::Error>;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let matches = app::app().get_matches();
    match matches.subcommand() {
        ("capture", Some(m)) => {
            capture::run(m)
        }
        ("", _) => {
            app::app().print_help()?;
            println!("");
            Ok(())
        }
        (unknown, _) => bail!("unrecognized command: {}", unknown),
    }
}
