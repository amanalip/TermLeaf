#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::Parser;
use termleaf::{cli::Cli, process};

fn main() -> ExitCode {
    let cli = Cli::parse();
    process::run(cli)
}
