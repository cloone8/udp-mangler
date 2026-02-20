//! Incoming packet listening

use core::error::Error;
use core::sync::atomic::{AtomicBool, Ordering};
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::mpsc::{SendError, Sender};
use std::time::Instant;

use arc_swap::ArcSwap;

use crate::{ManglerConfig, Packet};

/// The main function for the listener thread. The listener thread reads input packets from a UDP socket, and simply
/// forwards them to the [mangler thread](crate::mangle::mangle_main)
pub(crate) fn listen_main(
    config: Arc<ArcSwap<ManglerConfig>>,
    errs: Sender<Box<dyn Error + Send>>,
    socket: UdpSocket,
    to_mangler: Sender<Packet>,
    quit: Arc<AtomicBool>,
) {
    let mut buffer = Vec::new();

    while !quit.load(Ordering::Acquire) {
        buffer.clear();
        buffer.resize(config.load().buffer_size, 0);

        let (packet_size, sender_addr) = match socket.recv_from(&mut buffer) {
            Ok(packet_size) => packet_size,
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                // Retry loop
                continue;
            }
            Err(e) => {
                log::error!("Socket err: {e}");
                _ = errs.send(Box::new(e));
                break;
            }
        };

        if packet_size >= buffer.len() {
            // Packet might be truncated
            continue;
        }

        log::trace!("New UDP packet of size {packet_size} from {sender_addr}");

        let packet = Packet {
            send_timestamp: Instant::now(),
            content: Vec::from(&buffer[..packet_size]),
        };

        match to_mangler.send(packet) {
            Ok(val) => val,
            Err(SendError(_)) => {
                log::debug!("Listener thread returning because the to_mangler channel has closed");
                return;
            }
        };
    }
}
