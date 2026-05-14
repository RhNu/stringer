#![forbid(unsafe_code)]

use clap::Parser;
use miette::miette;
use stringer_cli::Cli;

#[tokio::main]
async fn main() -> miette::Result<()> {
    stringer_cli::run(Cli::parse())
        .await
        .map_err(|error| miette!("{error}"))
}
