use core::slice;

use anyhow::Context;
use vtd_protocol::command::{
    DriverCommandMetricsFlush,
    DriverCommandMetricsReportSend,
};

use crate::metrics;

pub fn metrics_report_send(command: &mut DriverCommandMetricsReportSend) -> anyhow::Result<()> {
    let payload =
        unsafe { slice::from_raw_parts(command.report_payload, command.report_payload_length) };

    metrics::add_record(
        command.get_report_type().unwrap_or("error"),
        str::from_utf8(payload).context("invalid payload encoding")?,
    );

    Ok(())
}

pub fn metrics_flush(command: &mut DriverCommandMetricsFlush) -> anyhow::Result<()> {
    command.queue_remaining = metrics::flush(command.blocking);
    Ok(())
}
