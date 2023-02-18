use shared::service::ServiceContext;
use std::{
    sync::Arc,
    sync::RwLock,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tokio::runtime::Handle;
use tracing::info;

pub(crate) struct Spinner {
    tokio_handle: Handle,
    shutdown: Arc<RwLock<bool>>,
    context: ServiceContext,
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
    pub fn new(context: ServiceContext) -> Self {
        Self {
            tokio_handle: Handle::current(),
            shutdown: Arc::new(RwLock::new(false)),
            context,
        }
    }
    pub fn start(self) -> SpinnerHandle {
        let shutdown_signal = self.shutdown.clone();
        let join = std::thread::Builder::new()
            .name("SPINNER".to_owned())
            .spawn(|| {
                self.run_forever();
            })
            .expect("Spinner thread failed to start!");

        SpinnerHandle {
            join,
            shutdown_signal,
        }
    }

    #[tracing::instrument(skip_all)]
    fn run_forever(self) {
        let config = self.context.load_config();
        let yield_max_duration =
            Duration::from_millis(config.scheduler.spinner_yield_max_ms);
        'tick_loop: loop {
            {
                let shutdown = self.shutdown.read().unwrap();
                if *shutdown {
                    break 'tick_loop;
                }
            }
            let instant = Instant::now();
            info!("[REMOVE ME] Tick...");
            self.tokio_handle.spawn(async move {
                info!("async-dispatch test");
            });

            let remaining =
                yield_max_duration.saturating_sub(instant.elapsed());
            if remaining != Duration::ZERO {
                // TODO: Consider using spin_sleep
                std::thread::sleep(remaining);
            }
        }
    }
}
