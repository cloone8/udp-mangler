//! Command line arguments and conversion

use core::net::SocketAddr;

use clap::Parser;

/// Args for the binary
#[derive(Debug, Clone, Parser)]
#[command(version, about)]
pub(crate) struct Args {
    /// The address on which the mangle server will listen for incoming UDP packets
    #[arg(short, long)]
    pub(crate) input: SocketAddr,

    /// The adress to which any UDP packets will be forwarded to, after mangling
    #[arg(short, long)]
    pub(crate) output: SocketAddr,

    #[arg(long, default_value_t = udp_mangler::ManglerConfig::default().buffer_size)]
    pub(crate) input_buffer_size: usize,

    #[arg(long, default_value_t = udp_mangler::ManglerConfig::default().max_payload_size)]
    pub(crate) max_payload_size: usize,

    #[arg(long, default_value_t = udp_mangler::ManglerConfig::default().loss_factor)]
    pub(crate) loss_factor: f64,
}

impl Args {
    pub(crate) fn validate(&self) -> bool {
        if self.input_buffer_size == 0 {
            eprintln!("Invalid input buffer size: {}", self.input_buffer_size);
            return false;
        }

        if self.max_payload_size == 0 {
            eprintln!("Invalid max payload size: {}", self.max_payload_size);
            return false;
        }

        if !(0.0..=1.0).contains(&self.loss_factor) {
            eprintln!("Invalid loss factor: {}", self.loss_factor);
            return false;
        }

        true
    }
}
