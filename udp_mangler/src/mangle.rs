use core::error::Error;
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};

use crate::{ManglerConfig, Packet};

pub fn mangle_main(
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

    while !quit.load(Ordering::Acquire) {
        let packet = ok_or_ret!(from_listener.recv());

        log::trace!("Mangling content: {:?}", packet);

        ok_or_ret!(to_forward.send(packet));
    }
}
