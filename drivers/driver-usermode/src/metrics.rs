use std::sync::RwLock;

use vtd_metrics::{
    MetricsClient,
    MK_INTERFACE_TYPE,
};

static METRICS: RwLock<Option<MetricsClient>> = RwLock::new(None);

pub fn maybe_init() {
    match METRICS.read() {
        Ok(inner) => {
            if inner.is_some() {
                return;
            }
        }
        Err(_) => {
            METRICS.clear_poison();
        }
    }

    if let Err(error) = self::init() {
        log::warn!("Metrics failed to initialize: {error}");
    }
}

pub fn init() -> anyhow::Result<()> {
    let metrics = vtd_metrics::create_instance(ureq::agent())?;

    metrics.add_record(MK_INTERFACE_TYPE, "usermode");
    *METRICS.write().unwrap() = Some(metrics);
    Ok(())
}

pub fn add_record(report_type: impl Into<String>, payload: impl Into<String>) {
    let Ok(metrics) = METRICS.read() else {
        return;
    };

    let Some(metrics) = &*metrics else {
        return;
    };

    metrics.add_record(report_type, payload);
}

pub fn flush(blocking: bool) -> usize {
    let Ok(metrics) = METRICS.read() else {
        return 0;
    };

    let Some(metrics) = &*metrics else {
        return 0;
    };

    metrics.flush(blocking)
}

pub fn shutdown() {
    let mut metrics = {
        let Ok(mut metrics) = METRICS.write() else {
            return;
        };

        let Some(metrics) = metrics.take() else {
            return;
        };

        metrics
    };

    metrics.shutdown();
}
