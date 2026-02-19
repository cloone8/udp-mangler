//! CLI for [udp_mangler]
//!
//!

use args::Args;
use clap::Parser;
use udp_mangler::{Mangler, ManglerConfig};

mod args;

fn main() {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Trace,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    let args = Args::parse();

    println!("{:#?}", args);

    let mangler = Mangler::new(args.input, args.output, ManglerConfig::default()).unwrap();

    mangler.block_on().unwrap();
}
