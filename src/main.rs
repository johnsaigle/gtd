mod commands;
mod gtd;
mod markdown;

use anyhow::Result;
use clap::Parser;
use commands::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    gtd::ensure_dirs()?;
    commands::run(cli)
}
