#![forbid(unsafe_code)]

use anyhow::Result;
use clap::Parser;
use termleaf::{app::App, cli::Cli, terminal};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let app = App::new(cli.book)?;
    terminal::run(app)
}
