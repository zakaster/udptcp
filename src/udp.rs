use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/*
    udp runs in blocking mode

    dependencies

        log module
            according to GPT, in rust 2018+ we get an implicit
            `extern crate log` so we don't need to import log
            anymore inside this file, log needs to be only
            initialized once and we can use it everywhere

        efram::egui
            uses ctx.repaint instead of sending messages via channel

*/

const READ_TIMEOUT: Duration = Duration::from_millis(100);

/// this is used when stuff happening inside worker thread
/// GUI is unable to update ontime because we cannot call
/// ctx.repaint() here, this is also for better decoupling
// pub enum UpdWorkerEvent {
//     Packet { src: SocketAddr, data: Vec<u8> },
//     Error(io::Error),
// }

pub struct Udp {
    // arc is for socket to be used in thread
    // the option, is a complicated topic, it is needed anyways
    // to represent a connected / unconnected socket, it is either
    // here or with the caller (GUI code), because when GUI initiates
    // there's no udpsocket, so we must wrap it with option
    // and this way it makes GUI code more complicated
    socket: Option<Arc<UdpSocket>>,

    is_running: Arc<AtomicBool>,

    // this is not actual state but desired value from GUI
    bc: bool,

    // event_tx: Sender<UpdWorkerEvent>,
    // event_rx: Receiver<UpdWorkerEvent>,
    worker: Option<JoinHandle<()>>,
}

impl Default for Udp {
    fn default() -> Self {
        Udp {
            socket: None,
            is_running: Arc::new(AtomicBool::new(false)),
            bc: false,
            worker: None,
        }
    }
}

impl Drop for Udp {
    fn drop(&mut self) {
        self.disconnect();
    }
}

impl Udp {
    /// unwrap an option, or return an Err
    #[allow(unused)]
    #[deprecated = "not really useful"]
    fn socket_ref(&self) -> io::Result<&UdpSocket> {
        // the map converts option<&arc<sock>> to option<&sock>
        self.socket.as_ref().map(|arc| arc.as_ref()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "UDP socket not initialized")
        })
    }

    pub fn is_up(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub fn connect_and_start(&mut self, sockaddr: String) -> io::Result<String> {
        let port = self.connect(sockaddr)?;
        self.start()?;
        Ok(port)
    }

    fn connect(&mut self, sockaddr: String) -> io::Result<String> {
        let socket = UdpSocket::bind(sockaddr)?;

        socket.set_broadcast(self.bc)?;
        socket.set_read_timeout(Some(READ_TIMEOUT))?;

        let port = socket.local_addr()?.port().to_string();

        let socket = Arc::new(socket);
        self.socket = Some(socket);

        log::info!("UDP socket bound: {:?}", self.socket);
        Ok(port)
    }

    /// terminate the worker and drop the binding
    pub fn disconnect(&mut self) {
        self.stop();
        self.socket.take();
        log::debug!("UDP disconnected, socket = {:?}", self.socket);
    }

    /// for this to work smoothly the GUI side needs to have
    /// a periodic repaint, otherwise pass in a ctx to run manual repaint
    pub fn start(&mut self) -> io::Result<()> {
        if self.is_running.load(Ordering::Relaxed) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "UDP worker already running",
            ));
        }

        // prepare clones for thread
        let Some(socket) = self.socket.as_ref().cloned() else {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "UDP not bound"));
        };
        let is_running = self.is_running.clone();

        // set state
        self.is_running.store(true, Ordering::Relaxed);

        // spawn workder thread
        // this runs forever until the flag has been set
        let handle = thread::spawn(move || {
            let mut buf = [0u8; 1024];
            while is_running.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buf) {
                    Ok((n, src)) => {
                        let bytes = buf[..n].to_vec();
                        let msg = String::from_utf8_lossy(&bytes);
                        log::info!("[UDP RECV] {:?} from {}", msg, src);
                    }
                    Err(e)
                        if e.kind() == io::ErrorKind::WouldBlock
                            || e.kind() == io::ErrorKind::TimedOut =>
                    {
                        // do nothing
                    }

                    // other errors, for example
                    // sending to an valid address + invalid (not used) port will get
                    // An existing connection was forcibly closed by the remote host. (os error 10054)
                    // also, no need to exit the thread in case of receiving error
                    Err(e) => {
                        log::error!("receiving error: {e}");
                    }
                }
            }
            // in theory the app should never reach here
            // udp worker thread can and should be closed only by
            // pressing the GUI button which calls disconnect()
            // if somehow this line is reached then we have a
            // problem of not releasing the socket
            log::debug!("UDP worker loop ended");
        });

        log::info!("UDP worker thread started: {:?}", handle.thread());
        self.worker = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.worker.take() {
            match handle.join() {
                Ok(()) => log::debug!("UDP worker thread terminated successfully"),
                Err(e) => log::error!("UDP worker thread termination error: {e:?}"),
            }
        }
    }

    /// why return Result:
    /// the GUI code will call this function
    /// and in case of failure we need to NOT set the toogle button
    pub fn toggle_broadcast(&mut self, flag: bool) -> io::Result<()> {
        // if udp bound
        if let Some(sock) = &self.socket {
            sock.set_broadcast(flag)?;
        }

        // if not bound, user can also flip the option
        Ok(())

        // // simulating a setting failure
        // Err(io::Error::new(io::ErrorKind::Other, "simulated"))
    }

    /// get the value of the SO_BROADCAST
    #[allow(dead_code)]
    #[deprecated = "might be useful if the state is controlled within"]
    fn broadcast_enabled(&self) -> bool {
        if let Some(sock) = &self.socket {
            return sock.broadcast().unwrap_or(false);
        }
        false
    }

    pub fn send_data_to(&self, msg: &str, to: &str) {
        let data = msg.as_bytes();

        if let Some(ref sock) = self.socket {
            match sock.send_to(&data, to) {
                Ok(_) => log::info!("[UDP SEND] {:?} to {}", msg, to),
                Err(e) => log::error!("error sending {:?} to {to}, {e}", msg),
            }
        } else {
            log::error!("UDP socket not initialized");
        }
    }

    // pub fn poll_events(&self) -> Vec<UpdWorkerEvent> {
    //     let mut out = Vec::new();
    //     while let Ok(ev) = self.event_rx.try_recv() {
    //         out.push(ev);
    //     }
    //     out
    // }
}
