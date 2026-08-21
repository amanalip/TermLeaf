use std::path::PathBuf;

use clap::Parser;

/// Read local books without leaving the terminal.
#[derive(Debug, Parser)]
#[command(name = "termleaf", version, about)]
pub struct Cli {
    /// Local book to open.
    #[arg(value_name = "BOOK")]
    pub book: Option<PathBuf>,

    /// Starting theme; overrides config.toml for this session.
    #[arg(
        long,
        value_name = "THEME",
        value_parser = ["dark", "light", "high-contrast", "monochrome", "paper"],
    )]
    pub theme: Option<String>,
}
