#![doc = include_str!("../../README.md")]

use core::error::Error;
use core::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvError, channel};
use std::thread::JoinHandle;

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
    config: ManglerConfig,
    listen_thread: Option<JoinHandle<()>>,
    mangler_thread: Option<JoinHandle<()>>,
    forward_thread: Option<JoinHandle<()>>,
    errs: Receiver<Box<dyn Error + Send>>,
    quit: Arc<AtomicBool>,
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub enum NewManglerErr {
    #[display("Error opening listener socket: {}", _0)]
    Listener(std::io::Error),

    #[display("Error opening forwarder socket: {}", _0)]
    Forwarder(std::io::Error),
}

impl Mangler {
    pub fn new(
        listen: SocketAddr,
        forward: SocketAddr,
        config: ManglerConfig,
    ) -> Result<Self, NewManglerErr> {
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
            errs: err_recv,
            quit,
        })
    }

    pub fn block_on(self) -> Result<(), Box<dyn Error>> {
        let result = self.errs.recv();

        match result {
            Ok(err) => {
                log::error!("Received error, stopping mangler: {err}");
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

        self.quit.store(true, Ordering::Release);

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

#[derive(Debug, Clone)]
pub struct ManglerConfig {
    pub buffer_size: usize,
    pub max_payload_size: usize,
    pub loss_factor: f64,
}

impl Default for ManglerConfig {
    fn default() -> Self {
        Self {
            buffer_size: u16::MAX as usize,
            max_payload_size: 1472,
            loss_factor: 0.005,
        }
    }
}

#[derive(Debug, Clone)]
struct Packet {
    content: Vec<u8>,
}
