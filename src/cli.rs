use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "bwww",
    about = "Build and preview the filesystem-first Bochen Shen site"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    New {
        title: String,
        #[arg(long)]
        slug: Option<String>,
    },
    Build {
        #[arg(long, default_value = "https://6ixgodd.github.io/")]
        site_url: String,
    },
    Check,
    Serve {
        #[arg(long, default_value_t = 8080)]
        port: u16,
        #[arg(long, default_value = "https://6ixgodd.github.io/")]
        site_url: String,
    },
}
