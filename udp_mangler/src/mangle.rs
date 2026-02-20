//! Packet mangling and UDP stream distortion

use core::error::Error;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SendError, Sender};
use std::time::Instant;

use arc_swap::ArcSwap;
use rand::RngExt;

use crate::{ByTimestamp, ManglerConfig, Packet};

/// Main function for the mangler thread.
/// The mangler thread takes the stream of input packets from the [listener thread](crate::listen::listen_main),
/// and distorts the stream in arbitrary ways. For example, it adds additional latency and jitter, and can randomly
/// drop packets
pub(crate) fn mangle_main(
    config: Arc<ArcSwap<ManglerConfig>>,
    _errs: Sender<Box<dyn Error + Send>>,
    from_listener: Receiver<Packet>,
    to_forward: Sender<Packet>,
    quit: Arc<AtomicBool>,
) {
    const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(100);

    let mut rng = rand::rng();
    let mut queue: BTreeSet<ByTimestamp> = BTreeSet::new();
    let mut next_queued: Instant = Instant::now() + DEFAULT_POLL_INTERVAL;

    while !quit.load(Ordering::Acquire) {
        let now = Instant::now();

        while let Some(next_packet) = queue.last()
            && next_packet.send_timestamp <= now
        {
            let to_send = queue.pop_last().unwrap();

            log::trace!("Forwarding packet: {:#?}", to_send.0);
            match to_forward.send(to_send.0) {
                Ok(val) => val,
                Err(SendError(_)) => {
                    log::debug!("Mangle thread returning because the forwarder channel was closed");
                    return;
                }
            };
        }

        let timeout = next_queued.duration_since(now);

        let mut packet = match from_listener.recv_timeout(timeout) {
            Ok(p) => p,
            Err(RecvTimeoutError::Timeout) => {
                next_queued = now + DEFAULT_POLL_INTERVAL;
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => {
                log::debug!("Mangle thread returning because the listener channel was closed");
                return;
            }
        };

        log::trace!("Mangling content: {:?}", packet);

        let config = config.load();

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

        if config.ping_secs != 0.0 {
            packet.send_timestamp += Duration::from_secs_f64(config.ping_secs)
        }

        if config.jitter_secs != 0.0 {
            let offset = rng.random_range::<f64, _>(0.0..=(config.jitter_secs));

            packet.send_timestamp += Duration::from_secs_f64(offset);
        }

        log::trace!("Inserting into queue: {:#?} (now: {now:?})", packet);
        queue.insert(packet.into());

        // Set the next "wake up" time to when the next packet is scheduled.
        // If no packet is scheduled, set a default interval to make sure we check the `quit` bool once
        // in a while
        next_queued = match queue.last() {
            Some(next) => next.send_timestamp,
            None => now + DEFAULT_POLL_INTERVAL,
        }
    }
}
