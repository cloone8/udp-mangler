use core::error::Error;
use core::sync::atomic::{AtomicBool, Ordering};
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::mpsc::Sender;

use crate::{ManglerConfig, Packet};

pub fn listen_main(
    config: ManglerConfig,
    errs: Sender<Box<dyn Error + Send>>,
    socket: UdpSocket,
    to_mangler: Sender<Packet>,
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

    let mut buffer = Vec::new();

    while !quit.load(Ordering::Acquire) {
        buffer.clear();
        buffer.resize(config.buffer_size, 0);

        let (packet_size, sender_addr) = match socket.recv_from(&mut buffer) {
            Ok(packet_size) => packet_size,
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                // Retry loop
                continue;
            }
            Err(e) => {
                _ = errs.send(Box::new(e));
                break;
            }
        };

        log::trace!("New UDP packet of size {packet_size} from {sender_addr}");

        to_mangler
            .send(Packet {
                content: Vec::from(&buffer[..packet_size]),
            })
            .expect("to_mangler channel dead");
    }
}
