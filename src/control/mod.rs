use crate::logs::LogsStore;
use std::future::Future;
use std::time::SystemTime;
use tokio::runtime::Runtime;
use tokio_metrics::{RuntimeMonitor, TaskMonitor};

pub struct Control {
    pub task_mon: TaskMonitor,
    pub runtime_mon: RuntimeMonitor,
    pub runtime: Runtime,
    pub logs: LogsStore,
}

impl Control {
    /// Creates a new Control that owns a single-threaded Tokio runtime, task and runtime monitors, and the provided logs store.
    ///
    /// The provided `LogsStore` is moved into the returned `Control` and will be used for periodic metrics logging.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::control::Control;
    /// use crate::logs::LogsStore;
    ///
    /// // Construct or obtain a LogsStore implementation and create the control
    /// let logs = LogsStore::new();
    /// let control = Control::new(logs);
    /// ```
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

    /// Spawn and run a future while attaching task monitoring instrumentation.
    ///
    /// Returns the future's output (`F::Output`).
    ///
    /// # Examples
    ///
    /// ```
    /// # // The following lines are hidden from docs and allow this example to compile in tests.
    /// # use crate::control::Control;
    /// # use crate::logs::LogsStore;
    /// # async fn __doc_example() {
    /// # let logs = LogsStore::new();
    /// # let control = Control::new(logs);
    /// let result = control.spawn(async { 42 }).await;
    /// assert_eq!(result, 42);
    /// # }
    /// ```
    pub async fn spawn<F>(&self, fut: F) -> F::Output
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        self.task_mon.instrument(fut).await
    }

    /// Starts a background task that periodically collects cumulative task metrics and writes them to the configured LogsStore.
    ///
    /// The background task samples metrics every 60 seconds and records them together with the current UNIX epoch seconds. If writing to the log store fails, an error is printed to stderr. This method returns after the background task has been spawned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tokio::runtime::Runtime;
    /// # // Assume `control` is an initialized `Control` from this crate.
    /// # async fn example(control: &crate::control::Control) {
    /// control.start_metrics_collection().await;
    /// # }
    /// ```
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
                    if let Err(err) =
                        logs.put(duration.as_secs(), format!("{:?}", metrics).into_bytes())
                    {
                        eprintln!("Failed to log metrics: {}", err);
                    }
                }
            }
        })
        .await
        .expect("failed to start metrics collection");
    }
    /// Shuts down the managed Tokio runtime.
    ///
    /// Consumes the `Control` and signals its runtime to stop executing background tasks; this call does not wait for the runtime to finish shutting down.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::control::Control;
    /// # use crate::logs::LogsStore;
    /// # async fn example() {
    /// let logs = LogsStore::new();
    /// let control = Control::new(logs);
    /// control.stop().await;
    /// # }
    /// ```
    pub async fn stop(self) {
        self.runtime.shutdown_background();
    }
}
