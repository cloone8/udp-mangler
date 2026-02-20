//! Command line arguments and conversion

use core::net::SocketAddr;

use clap::Parser;
use udp_mangler::ManglerConfig;

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

    /// The log level used
    #[arg(short, long, default_value_t = if cfg!(debug_assertions) { simplelog::LevelFilter::Debug } else { simplelog::LevelFilter::Info })]
    pub(crate) verbosity: simplelog::LevelFilter,

    /// The size if the input buffer. Does not influence mangling, but any larger packets are always dropped
    #[arg(long, default_value_t = udp_mangler::ManglerConfig::default().buffer_size)]
    pub(crate) input_buffer_size: usize,

    /// The maximum size of the incoming packet payload before the mangler either drops or fragments them
    #[arg(long, default_value_t = udp_mangler::ManglerConfig::default().max_payload_size)]
    pub(crate) max_payload_size: usize,

    /// The factor of packets that are randomly dropped by the mangler
    #[arg(long, default_value_t = udp_mangler::ManglerConfig::default().loss_factor)]
    pub(crate) loss_factor: f64,

    /// Additional ping to add, in milliseconds
    #[arg(long, default_value_t = 0)]
    pub(crate) ping: usize,

    /// Additional jitter to add, in milliseconds
    #[arg(long, default_value_t = 0)]
    pub(crate) jitter: usize,
}

impl Args {
    /// Validates the arguments and returns a [ManglerConfig] if valid
    pub(crate) fn validate(&self) -> Result<ManglerConfig, ()> {
        if self.input_buffer_size == 0 {
            eprintln!("Invalid input buffer size: {}", self.input_buffer_size);
            return Err(());
        }

        if self.max_payload_size == 0 {
            eprintln!("Invalid max payload size: {}", self.max_payload_size);
            return Err(());
        }

        if !(0.0..=1.0).contains(&self.loss_factor) {
            eprintln!("Invalid loss factor: {}", self.loss_factor);
            return Err(());
        }

        Ok(ManglerConfig {
            buffer_size: self.input_buffer_size,
            max_payload_size: self.max_payload_size,
            loss_factor: self.loss_factor,
            ping_secs: (self.ping as f64) / 1000.0,
            jitter_secs: (self.jitter as f64) / 1000.0,
        })
    }
}
