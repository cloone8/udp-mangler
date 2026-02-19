use core::error::Error;
use core::sync::atomic::{AtomicBool, Ordering};
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use crate::{ManglerConfig, Packet};

pub fn forward_main(
    config: ManglerConfig,
    errs: Sender<Box<dyn Error + Send>>,
    socket: UdpSocket,
    from_mangler: Receiver<Packet>,
    quit: Arc<AtomicBool>,
) {
    macro_rules! ok_or_ret {
        ($result:expr) => {
            match $result {
                Ok(val) => val,
                Err(e) => {
                    _ = errs.send(Box::new(e));
                    break;
                }
            }
        };
    }

    log::info!("Forwarding to address: {}", socket.peer_addr().unwrap());

    let mut packet: Option<Packet> = None;

    while !quit.load(Ordering::Acquire) {
        if packet.is_none() {
            packet = Some(ok_or_ret!(from_mangler.recv()));
        }

        let cur_packet = packet.clone().unwrap();

        let num_written = match socket.send(&cur_packet.content) {
            Ok(num_written) => num_written,
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                // Retry loop
                continue;
            }
            Err(e) => {
                _ = errs.send(Box::new(e));
                break;
            }
        };

        packet = None;

        log::trace!("Forwarded {num_written} bytes");
    }
}
