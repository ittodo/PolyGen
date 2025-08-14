use anyhow::Result;
use clap::Parser;
use PolyGen::{run, Cli};

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}