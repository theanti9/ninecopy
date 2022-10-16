use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ninecopy")]
#[command(author = "theanti9")]
#[command(version = "1.0")]
#[command(about = "Fast, multithreaded directory copy utility", long_about = None)]
pub struct Args {
    /// The folder you want to copy.
    ///
    /// e.x. "C:\MyFolder"
    #[arg(value_name = "SOURCE")]
    pub src: PathBuf,

    /// The location you want to copy SOURCE to.
    ///
    /// e.x. "D:\MyFolder"
    #[arg(value_name = "DESTINATION")]
    pub dst: PathBuf,

    /// Overwrite existing files.
    ///
    /// If this is false, the process will exit if existing files at the destination are encountered.
    #[arg(short, long)]
    pub overwrite: bool,

    /// Periodically log progress.
    #[arg(short, long)]
    pub progress: bool,

    /// The number of threads to use for search and copy.
    ///
    /// Defaults to one per core.
    ///
    /// Transfers with mostly large files may benefit from thread counts higher than one per core, depending on the core count and disk throughput.
    #[arg(short, long)]
    pub threads: Option<usize>,
}
