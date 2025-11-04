use core::{
    self,
    time::Duration,
};
use std::{
    sync::{
        Arc,
        Mutex,
        mpsc::{
            self,
            RecvTimeoutError,
        },
    },
    thread::{
        self,
    },
    time::{
        Instant,
        SystemTime,
        UNIX_EPOCH,
    },
};

use obfstr::obfstr;

use super::data::MetricsRecord;
use crate::{
    KEY_METRICS_INTERNAL,
    device,
    error::MetricsResult,
    queue::RecordQueue,
    sender::MetricsSender,
};

const SUBMIT_BACKOFF_INTERVALS: [Duration; 5] = [
    Duration::from_secs(5),   /* 5 sec */
    Duration::from_secs(30),  /* 30 sec */
    Duration::from_secs(60),  /* 1 min */
    Duration::from_secs(90),  /* 1.5 min */
    Duration::from_secs(120), /* 2 min */
];

#[derive(Copy, Clone, Debug)]
enum SendTimerMode {
    Normal,
    Backoff,
    BackoffForced,
}

enum WorkerAction {
    FlushQueue,
    FlushShutdown,
}

pub struct WorkerContext {
    sender: MetricsSender,

    record_queue: Arc<Mutex<RecordQueue>>,
    submit_interval: Duration,

    send_deadline: Instant,
    send_timer_mode: SendTimerMode,

    action_rx: mpsc::Receiver<WorkerAction>,
    flush_tx: mpsc::SyncSender<()>,

    /// Number of concurrent submit failures    
    backoff_level: usize,

    shutdown: bool,
}

impl WorkerContext {
    fn worker_loop(&mut self) {
        log::trace!("Metrics worker started");
        while !self.shutdown {
            let action = match self.action_rx.recv_deadline(self.send_deadline) {
                Ok(action) => Some(action),
                Err(RecvTimeoutError::Disconnected) => Some(WorkerAction::FlushShutdown),
                Err(RecvTimeoutError::Timeout) => None,
            };

            if let Some(action) = action {
                match action {
                    WorkerAction::FlushShutdown => {
                        /* shutdown as soon we're finished */
                        self.shutdown = true;
                    }
                    WorkerAction::FlushQueue => {
                        if matches!(self.send_timer_mode, SendTimerMode::BackoffForced) {
                            /* We're on a force backoff. Nothing we can do. */
                            continue;
                        }
                    }
                }
            }

            loop {
                let result = self.submit_queue();
                self.handle_submit_result(&result);

                if !matches!(result, QueueSubmitResult::Success) {
                    /* no more submits can be done */
                    break;
                }
            }

            let _ = self.flush_tx.send(());
        }

        let _ = self.flush_tx.send(());
        log::trace!("Metrics worker ended");
    }

    fn handle_submit_result(&mut self, result: &QueueSubmitResult) {
        match result {
            QueueSubmitResult::QueueEmpty => {
                /* no more records to submit */
                self.send_deadline = Instant::now() + self.submit_interval;
            }
            QueueSubmitResult::Success => {
                if !matches!(self.send_timer_mode, SendTimerMode::Normal) {
                    /* Server accepts records again, juhu :) */
                    log::debug!("Switched into normal timer mode (submit success).");
                    self.send_timer_mode = SendTimerMode::Normal;
                }
            }
            QueueSubmitResult::BackoffServer(duration) => {
                log::trace!("Switching into forced backoff for {:?}", duration);

                /* Reset the backoff level as after cleating the backoff received by the server we should not have any more backoffs */
                self.backoff_level = 0;

                self.send_deadline = Instant::now() + *duration;
                self.send_timer_mode = SendTimerMode::BackoffForced;
            }
            QueueSubmitResult::BackoffFailure => {
                let backoff =
                    SUBMIT_BACKOFF_INTERVALS[self.backoff_level % SUBMIT_BACKOFF_INTERVALS.len()];
                log::trace!(
                    "Switching into backoff with level {} ({:#?})",
                    self.backoff_level,
                    backoff
                );
                self.backoff_level += 1;

                self.send_deadline = Instant::now() + backoff;
                self.send_timer_mode = SendTimerMode::Backoff;
            }
        }
    }

    fn submit_queue(&mut self) -> QueueSubmitResult {
        let report_records = {
            let mut queue = self.record_queue.lock().unwrap();
            queue.dequeue_for_report()
        };

        let Some(mut report_records) = report_records else {
            return QueueSubmitResult::QueueEmpty;
        };

        let report_records_slice = report_records.make_contiguous();
        match self.sender.submit_records(report_records_slice) {
            Ok(_) => {
                log::trace!("records submitted {}", report_records.len());
                return QueueSubmitResult::Success;
            }
            Err(info) => {
                log::trace!(
                    "Failed to submit {} reports: {:#}. Retry: {:?}, drop all: {}, submitted reports: {:?}",
                    report_records_slice.len(),
                    info.reason,
                    info.retry_delay,
                    info.drop_records,
                    info.records_submitted
                );

                if !info.drop_records {
                    let mut queue = self.record_queue.lock().unwrap();
                    queue.enqueue_failed(
                        report_records
                            .into_iter()
                            .filter(|entry| !info.records_submitted.contains(&entry.seq_no)),
                    );
                }

                return if let Some(retry_delay) = info.retry_delay {
                    QueueSubmitResult::BackoffServer(Duration::from_secs(retry_delay as u64))
                } else {
                    QueueSubmitResult::BackoffFailure
                };
            }
        }
    }
}

enum QueueSubmitResult {
    Success,
    QueueEmpty,
    BackoffServer(Duration),
    BackoffFailure,
}

pub struct MetricsClient {
    record_queue: Arc<Mutex<RecordQueue>>,

    worker_action_tx: mpsc::Sender<WorkerAction>,
    flush_rx: Option<Mutex<mpsc::Receiver<()>>>,
}

impl MetricsClient {
    pub fn new(http_agent: ureq::Agent) -> MetricsResult<Self> {
        let server_url = if let Some(value) = option_env!("METRICS_TARGET") {
            value.to_string()
        } else {
            obfstr!("https://metrics.valth.run/api/v1/report").to_string()
        };

        let record_queue = Arc::new(Mutex::new(RecordQueue::new()));

        let (worker_action_tx, worker_action_rx) = mpsc::channel();
        let (flush_tx, flush_rx) = mpsc::sync_channel(0x01);

        thread::spawn({
            let sender = MetricsSender::new(http_agent, server_url, device::resolve_info()?)?;
            let record_queue = record_queue.clone();

            move || {
                let mut ctx = WorkerContext {
                    sender,
                    record_queue,

                    submit_interval: Duration::from_secs(2 * 60),

                    action_rx: worker_action_rx,
                    flush_tx,

                    send_deadline: Instant::now() + Duration::from_secs(25),
                    send_timer_mode: SendTimerMode::Normal,

                    backoff_level: 0,
                    shutdown: false,
                };

                WorkerContext::worker_loop(&mut ctx);
            }
        });

        let instance = Self {
            record_queue,

            worker_action_tx,
            flush_rx: Some(Mutex::new(flush_rx)),
        };
        instance.add_record(KEY_METRICS_INTERNAL, "startup");
        Ok(instance)
    }

    pub fn add_record(&self, report_type: impl Into<String>, payload: impl Into<String>) {
        let record = MetricsRecord {
            report_type: report_type.into(),
            payload: payload.into(),
            timestamp: self::get_system_time_precise_as_filetime(),
            uptime: device::get_tick_count64(),
            seq_no: 0,
        };

        let mut record_queue = self.record_queue.lock().unwrap();
        record_queue.add_record(record);

        if record_queue.queue_size() >= 1_000 && record_queue.queue_size() % 100 == 0 {
            /* force flush */
            let _ = self.worker_action_tx.send(WorkerAction::FlushQueue);
        }
    }

    pub fn flush(&self) {
        let Some(flush_rx) = &self.flush_rx else {
            /* shutdown already done */
            return;
        };

        let flush_rx = flush_rx.lock().unwrap();
        while let Ok(_) = flush_rx.try_recv() {}

        let _ = self.worker_action_tx.send(WorkerAction::FlushQueue);
        let _ = flush_rx.recv();
    }

    pub fn shutdown(&mut self) {
        let Some(flush_rx) = self.flush_rx.take() else {
            /* shutdown already done */
            return;
        };
        let flush_rx = flush_rx.into_inner().unwrap();
        while let Ok(_) = flush_rx.try_recv() {}

        log::trace!("Requesting flush & shutdown");
        self.add_record(KEY_METRICS_INTERNAL, "shutdown");

        let _ = self.worker_action_tx.send(WorkerAction::FlushShutdown);
        if flush_rx.recv_timeout(Duration::from_secs(5)).is_err() {
            log::warn!("Metrics thread timed out");
        }

        log::trace!("Shutdown finished");
    }
}

impl Drop for MetricsClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Returns a Windows-style FILETIME value (100-ns intervals since Jan 1, 1601 UTC)
/// equivalent to `GetSystemTimePreciseAsFileTime()`.
fn get_system_time_precise_as_filetime() -> u64 {
    // FILETIME epoch is 1601-01-01, UNIX epoch is 1970-01-01.
    const WINDOWS_TICK: u64 = 10_000_000; // 1 second = 10^7 FILETIME ticks
    const SEC_TO_UNIX_EPOCH: u64 = 11_644_473_600; // seconds between 1601 and 1970

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!");

    // Convert UNIX epoch to FILETIME epoch
    (now.as_secs() + SEC_TO_UNIX_EPOCH) * WINDOWS_TICK + (now.subsec_nanos() as u64 / 100)
}

pub fn create_instance(http_agent: ureq::Agent) -> MetricsResult<MetricsClient> {
    MetricsClient::new(http_agent)
}

#[cfg(test)]
mod test {
    #[test]
    fn basic_function() {
        let _ = env_logger::try_init();

        let agent = ureq::agent();
        let mut instance = super::create_instance(agent).unwrap();

        instance.add_record("test-request", "some-payload");
        instance.flush();
        instance.shutdown();
    }
}
