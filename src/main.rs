mod cli;
mod commands;
mod content;
mod error;
mod rst;
mod site;

use crate::cli::{Cli, Command};
use crate::error::Result;
use clap::Parser;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::New { title, slug } => commands::new::run(&title, slug.as_deref()),
        Command::Build { site_url } => commands::build::run(&site_url),
        Command::Check => commands::check::run(),
        Command::Serve { port, site_url } => commands::serve::run(port, &site_url),
    }
}
