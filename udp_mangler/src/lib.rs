#![doc = include_str!("../README.md")]

use core::error::Error;
use core::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use std::net::UdpSocket;
use std::sync::mpsc::{Receiver, RecvError, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use arc_swap::ArcSwap;
use forward::forward_main;
use listen::listen_main;
use mangle::mangle_main;

mod forward;
mod listen;
mod mangle;

/// The main entrypoint for the [udp_mangler](crate) library. Create
/// an instance with [Mangler::new]
#[derive(Debug)]
pub struct Mangler {
    /// The current configuration
    config: Arc<ArcSwap<ManglerConfig>>,

    /// Handle to the listen thread
    listen_thread: Option<JoinHandle<()>>,

    /// Handle to the mangler thread
    mangler_thread: Option<JoinHandle<()>>,

    /// Handle to the forward thread
    forward_thread: Option<JoinHandle<()>>,

    /// Receiver that gets fatal errors encountered by the
    /// worker threads
    errs: Mutex<Receiver<Box<dyn Error + Send>>>,

    /// A flag that can be set to have the worker threads quit
    quit: Arc<AtomicBool>,
}

/// Error while constructing a new mangler
#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum NewManglerErr {
    /// Could not open the UDP socket that is used for listening for incoming packets
    #[display("Error opening listener socket: {}", _0)]
    Listener(std::io::Error),

    /// Could not open the UDP socket that is used for forwarding the mangled packets
    #[display("Error opening forwarder socket: {}", _0)]
    Forwarder(std::io::Error),
}

impl Mangler {
    /// Creates a new mangler that listens for incoming packets on `listen`, then mangles them
    /// according to the given `config`, and then finally forwards them to `forward`
    pub fn new(
        listen: SocketAddr,
        forward: SocketAddr,
        config: ManglerConfig,
    ) -> Result<Self, NewManglerErr> {
        let config = Arc::new(ArcSwap::from_pointee(config));
        let quit = Arc::new(AtomicBool::new(false));

        let (to_mangler_send, to_mangler_recv) = channel::<Packet>();
        let (to_forward_send, to_forward_recv) = channel::<Packet>();
        let (err_send, err_recv) = channel::<Box<dyn Error + Send>>();

        let listener_socket = UdpSocket::bind(listen).map_err(NewManglerErr::Listener)?;

        listener_socket
            .set_read_timeout(Some(Duration::from_secs_f64(0.1)))
            .expect("Failed to set read timeout on listener socket");

        let forwarder_socket = UdpSocket::bind(if forward.is_ipv4() {
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))
        } else {
            SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0))
        })
        .map_err(NewManglerErr::Forwarder)?;

        forwarder_socket
            .connect(forward)
            .map_err(NewManglerErr::Forwarder)?;

        forwarder_socket
            .set_write_timeout(Some(Duration::from_secs_f64(0.1)))
            .expect("Failed to set write timeout on forwarder socket");

        let quit_cloned = quit.clone();
        let cloned_config = config.clone();
        let err_send_cloned = err_send.clone();
        let listen_thread = std::thread::spawn(move || {
            listen_main(
                cloned_config,
                err_send_cloned,
                listener_socket,
                to_mangler_send,
                quit_cloned,
            )
        });

        let quit_cloned = quit.clone();
        let cloned_config = config.clone();
        let err_send_cloned = err_send.clone();
        let mangler_thread = std::thread::spawn(move || {
            mangle_main(
                cloned_config,
                err_send_cloned,
                to_mangler_recv,
                to_forward_send,
                quit_cloned,
            )
        });

        let quit_cloned = quit.clone();
        let cloned_config = config.clone();
        let err_send_cloned = err_send.clone();
        let forward_thread = std::thread::spawn(move || {
            forward_main(
                cloned_config,
                err_send_cloned,
                forwarder_socket,
                to_forward_recv,
                quit_cloned,
            )
        });

        Ok(Self {
            config,
            listen_thread: Some(listen_thread),
            mangler_thread: Some(mangler_thread),
            forward_thread: Some(forward_thread),
            errs: Mutex::new(err_recv),
            quit,
        })
    }

    /// Updates the config used for mangling
    pub fn update_config(&self, new_config: ManglerConfig) {
        self.config.store(Arc::new(new_config));
    }

    /// Stops the mangler threads gracefully.
    /// The threads themselves are not guaranteed to be done until after this [Mangler] is [dropped](drop)
    pub fn stop(&self) {
        self.quit.store(true, Ordering::Release);
    }

    /// Blocks the main thread until the mangler stops by itself
    pub fn wait_until_complete(&self) -> Result<(), Box<dyn Error>> {
        let result = self.errs.lock().unwrap().recv();

        match result {
            Ok(err) => {
                log::error!("Received error: {err}");
                Err(err)
            }
            Err(RecvError) => {
                // Channel was closed before any error was returned.
                // This is the "good" scenario
                Ok(())
            }
        }
    }
}

impl Drop for Mangler {
    fn drop(&mut self) {
        log::info!("Mangler dropped, stopping threads...");

        self.stop();

        _ = self.wait_until_complete();

        if let Some(th) = self.listen_thread.take() {
            th.join().expect("Failed to join listener thread");
        }

        if let Some(th) = self.mangler_thread.take() {
            th.join().expect("Failed to join mangler thread");
        }

        if let Some(th) = self.forward_thread.take() {
            th.join().expect("Failed to join forward thread");
        }
    }
}

/// The configuration for a [Mangler]
#[derive(Debug, Clone, PartialEq)]
pub struct ManglerConfig {
    /// The internal buffer size used to receive packets.
    /// Does not affect mangling functionality, but any packets larger than this
    /// are dropped without any processing
    pub buffer_size: usize,

    /// The maximum payload size of a UDP packet before it is either dropped by or fragmented by
    /// the mangler
    pub max_payload_size: usize,

    /// The factor (between 0.0 and 1.0 inclusive) of randomly dropped packets
    pub loss_factor: f64,

    /// Additional ping to add
    pub ping_secs: f64,

    /// Additional jitter to add
    pub jitter_secs: f64,
}

impl Default for ManglerConfig {
    fn default() -> Self {
        Self {
            buffer_size: u16::MAX as usize,
            max_payload_size: 1472,
            loss_factor: 0.005,
            ping_secs: 0.050,   // 50 ms
            jitter_secs: 0.020, // 20 ms
        }
    }
}

/// A received UDP packet, in the process of being mangled
#[derive(Debug, Clone)]
struct Packet {
    /// The timestamp at which point this packet should be sent out
    send_timestamp: Instant,

    /// The raw packet payload
    content: Vec<u8>,
}

/// Wrapper struct to sort [Packets](Packet) by their outgoing timestamp
#[derive(Debug, Clone)]
struct ByTimestamp(Packet);

impl From<Packet> for ByTimestamp {
    fn from(value: Packet) -> Self {
        Self(value)
    }
}

impl Deref for ByTimestamp {
    type Target = Packet;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ByTimestamp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PartialEq for ByTimestamp {
    fn eq(&self, other: &Self) -> bool {
        self.0.send_timestamp == other.0.send_timestamp
    }
}

impl Eq for ByTimestamp {}

#[allow(clippy::non_canonical_partial_ord_impl, reason = "Forward to Instant")]
impl PartialOrd for ByTimestamp {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.0.send_timestamp.partial_cmp(&other.0.send_timestamp)
    }
}

impl Ord for ByTimestamp {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.send_timestamp.cmp(&other.0.send_timestamp)
    }
}
