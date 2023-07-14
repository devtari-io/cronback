use tokio::sync::broadcast;
/// The `Shutdown` struct listens for the signal and tracks that the signal has
/// been received. Callers may query for whether the shutdown signal has been
/// received or not.
pub struct Shutdown {
    /// `true` if the shutdown signal has been received
    shutdown: bool,

    notify: broadcast::Sender<()>,
    /// The receive half of the channel used to listen for shutdown.
    watch: broadcast::Receiver<()>,
}

impl Clone for Shutdown {
    fn clone(&self) -> Self {
        let notify = self.notify.clone();
        let watch = notify.subscribe();
        Self {
            shutdown: self.shutdown,
            notify,
            watch,
        }
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        let (notify, watch) = broadcast::channel(1);
        Self {
            shutdown: false,
            notify,
            watch,
        }
    }
}

impl Shutdown {
    /// Create a new `Shutdown` backed by the given `broadcast::Receiver`.
    pub fn new(notify: broadcast::Sender<()>) -> Shutdown {
        let watch = notify.subscribe();
        Shutdown {
            shutdown: false,
            notify,
            watch,
        }
    }

    /// Returns `true` if the shutdown signal has been received.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub async fn recv(&mut self) {
        // If the shutdown signal has already been received, then return
        // immediately.
        if self.shutdown {
            return;
        }

        // Cannot receive a "lag error" as only one value is ever sent.
        let _ = self.watch.recv().await;

        // Remember that the signal has been received.
        self.shutdown = true;
    }

    pub fn broadcast_shutdown(&mut self) {
        self.notify.send(()).unwrap();
    }
}
