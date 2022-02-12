use std::path::PathBuf;

use clap::AppSettings::UseLongFormatForHelpSubcommand;
use clap::{Args, Parser, Subcommand};
use log::LevelFilter;
use bagr::bagit::{create_bag};

// TODO expand docs

/// A CLI for interacting with BagIt bags
#[derive(Debug, Parser)]
#[clap(name = "bagr", author = "Peter Winckles <pwinckles@pm.me>", version)]
#[clap(setting(UseLongFormatForHelpSubcommand))]
pub struct BagrArgs {
    /// Absolute or relative path to the bag's base directory
    ///
    /// By default, this is the current directory.
    #[clap(short, long, value_name = "BAG_PATH")]
    pub bag_path: Option<PathBuf>,

    /// Suppress error messages and other command specific logging
    #[clap(short, long)]
    pub quiet: bool,

    /// Increase log level
    #[clap(short = 'V', long)]
    pub verbose: bool,

    /// Disable all output styling
    #[clap(short = 'S', long)]
    pub no_styles: bool,

    /// Subcommand to execute
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[clap(name = "bag")]
    Bag(BagCmd),
}

/// Create a new bag
#[derive(Args, Debug)]
pub struct BagCmd {}

fn main() {
    let mut args = BagrArgs::parse();

    let log_level = if args.quiet {
        LevelFilter::Off
    } else if args.verbose {
        LevelFilter::Info
    } else {
        LevelFilter::Warn
    };

    env_logger::builder()
        .filter_level(log_level)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .init();

    // If the output is being piped then we should disable styling
    if atty::isnt(atty::Stream::Stdout) {
        args.no_styles = true;
    }

    // TODO
    create_bag(".").unwrap();
}
