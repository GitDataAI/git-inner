use std::future::Future;
use std::time::SystemTime;
use tokio::runtime::Runtime;
use tokio_metrics::{RuntimeMonitor, TaskMonitor};
use crate::logs::LogsStore;

pub struct Control {
    pub task_mon: TaskMonitor,
    pub runtime_mon: RuntimeMonitor,
    pub runtime: Runtime,
    pub logs: LogsStore,
}

impl Control {
    pub fn new(logs_store: LogsStore) -> Self {
        let task_mon = TaskMonitor::builder().build();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");

        let runtime_mon = RuntimeMonitor::new(runtime.handle());
        Control {
            task_mon,
            runtime_mon,
            runtime,
            logs: logs_store,
        }
    }

    /// Spawn a task with monitoring instrumentation
    pub async fn spawn<F>(&self, fut: F) -> F::Output
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.task_mon.instrument(fut).await
    }

    /// Start collecting metrics periodically
    pub async fn start_metrics_collection(&self) {
        let task_metrics = self.task_mon.clone();
        let logs = self.logs.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await;
            loop {
                interval.tick().await;
                let metrics = task_metrics.cumulative();
                if let Ok(duration) = SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    if let Err(err) = logs.put(
                        duration.as_secs(),
                        format!("{:?}", metrics).into_bytes()
                    ) {
                        eprintln!("Failed to log metrics: {}", err);
                    }
                }
            }
        }).await.expect("failed to start metrics collection");
    }
    pub async fn stop(self) {
        self.runtime.shutdown_background();
    }
}