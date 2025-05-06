use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub prompt: Option<String>,

    #[arg(
        help = "Files to process. Reads from stdin if no files are provided. Use '-' to read from stdin within a list of files."
    )]
    pub files: Vec<PathBuf>,

    #[arg(
        long,
        help = "Create a default configuration file if it doesn't exist."
    )]
    pub init_config: bool,

    #[arg(short, long, help = "Enable verbose output for debugging.")]
    pub verbose: bool,
}
