use std::path::PathBuf;

use clap::Parser;

/// Read local books without leaving the terminal.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// Local book to open.
    #[arg(value_name = "BOOK")]
    pub book: Option<PathBuf>,
}
