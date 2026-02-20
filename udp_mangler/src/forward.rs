//! Post-mangle packet forwarding

use core::error::Error;
use core::sync::atomic::{AtomicBool, Ordering};
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvError, Sender};

use arc_swap::ArcSwap;

use crate::{ManglerConfig, Packet};

/// The main function for the forward thread. The forward thread takes a stream of mangled
/// packets from the [mangle thread](crate::mangle::mangle_main), and simply forwards them
/// to the target address
pub(crate) fn forward_main(
    _config: Arc<ArcSwap<ManglerConfig>>,
    errs: Sender<Box<dyn Error + Send>>,
    socket: UdpSocket,
    from_mangler: Receiver<Packet>,
    quit: Arc<AtomicBool>,
) {
    log::info!("Forwarding to address: {}", socket.peer_addr().unwrap());

    let mut packet: Option<Packet> = None;

    while !quit.load(Ordering::Acquire) {
        if packet.is_none() {
            packet = Some(match from_mangler.recv() {
                Ok(p) => p,
                Err(RecvError) => {
                    log::debug!("Forward thread returning because the mangler channel was closed");
                    return;
                }
            });
        }

        let cur_packet = packet.clone().unwrap();

        let num_written = match socket.send(&cur_packet.content) {
            Ok(num_written) => num_written,
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

        packet = None;

        log::trace!("Forwarded {num_written} bytes");
    }
}
