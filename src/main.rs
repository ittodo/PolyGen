use anyhow::Result;
use clap::Parser;
use polygen::{run, Cli};

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}
