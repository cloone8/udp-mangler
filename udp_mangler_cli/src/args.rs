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
}
