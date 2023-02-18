use std::{
    sync::Arc,
    sync::RwLock,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tokio::runtime::Handle;
use tracing::info;

static TICK_DELAY: Duration = Duration::from_millis(500);

pub(crate) struct Spinner {
    // data: Vec<String>,
    tokio_handle: Handle,
    shutdown: Arc<RwLock<bool>>,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            tokio_handle: Handle::current(),
            shutdown: Arc::new(RwLock::new(false)),
        }
    }
}

pub(crate) struct SpinnerHandle {
    join: JoinHandle<()>,
    shutdown_signal: Arc<RwLock<bool>>,
}

impl SpinnerHandle {
    pub fn shutdown(self) {
        info!("Attempting to shutdown spinner");
        {
            let mut guard = self.shutdown_signal.write().unwrap();
            *guard = true;
        }
        info!("Waiting for spinner to shutdown");
        let _ = self.join.join();
        info!("Spinner terminated!");
    }
}

impl Spinner {
    pub fn start(self) -> SpinnerHandle {
        let shutdown_signal = self.shutdown.clone();
        let join = std::thread::spawn(|| {
            self.run_forever();
        });

        SpinnerHandle {
            join,
            shutdown_signal,
        }
    }

    fn run_forever(self) {
        'tick_loop: loop {
            {
                let shutdown = self.shutdown.read().unwrap();
                if *shutdown {
                    break 'tick_loop;
                }
            }
            let instant = Instant::now();
            println!("Tick...");
            self.tokio_handle.spawn(async move {
                println!("I AM ASYNC");
            });

            std::thread::sleep(Duration::from_millis(400));

            let remaining = TICK_DELAY.saturating_sub(instant.elapsed());
            if remaining != Duration::ZERO {
                println!("sleep for {:?}", remaining);
                std::thread::sleep(remaining);
            } else {
                println!("skip sleep");
            }
        }
    }
}
