use core::error::Error;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use rand::RngExt;

use crate::{ManglerConfig, Packet};

pub(crate) fn mangle_main(
    config: ManglerConfig,
    errs: Sender<Box<dyn Error + Send>>,
    from_listener: Receiver<Packet>,
    to_forward: Sender<Packet>,
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

    let mut rng = rand::rng();

    while !quit.load(Ordering::Acquire) {
        let packet = ok_or_ret!(from_listener.recv());

        log::trace!("Mangling content: {:?}", packet);

        if packet.content.len() > config.max_payload_size {
            log::trace!(
                "Dropping packet with size above maximum: {}, max {}",
                packet.content.len(),
                config.max_payload_size
            );
            continue;
        }

        if config.loss_factor != 0.0 && rng.random::<f64>() < config.loss_factor {
            log::trace!("Dropping packet randomly due to loss factor");
            continue;
        }

        ok_or_ret!(to_forward.send(packet));
    }
}
