#![doc = include_str!("../README.md")]

use std::process::ExitCode;
use std::sync::Arc;

use args::Args;
use clap::Parser;
use udp_mangler::Mangler;

mod args;

fn main() -> ExitCode {
    let args = Args::parse();

    simplelog::TermLogger::init(
        args.verbosity,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    let Ok(mangler_config) = args.validate() else {
        return ExitCode::FAILURE;
    };

    let mangler = Arc::new(Mangler::new(args.input, args.output, mangler_config).unwrap());

    let mangler_cloned = mangler.clone();

    // A handler is useful, but it only does a graceful shutdown so it's not essential
    _ = ctrlc::set_handler(move || mangler_cloned.stop());

    mangler.wait_until_complete().unwrap();

    ExitCode::SUCCESS
}
