use std::collections::VecDeque;

use obfstr::obfstr;

use crate::data::MetricsRecord;

pub struct RecordQueue {
    entry_sequence_no: u32,
    pending_entries: VecDeque<MetricsRecord>,
}

impl RecordQueue {
    pub fn new() -> Self {
        Self {
            entry_sequence_no: 0,
            pending_entries: Default::default(),
        }
    }

    fn next_sequence_id(&mut self) -> u32 {
        self.entry_sequence_no = self.entry_sequence_no.wrapping_add(1);
        self.entry_sequence_no
    }

    pub fn queue_size(&self) -> usize {
        self.pending_entries.len()
    }

    pub fn add_record(&mut self, mut record: MetricsRecord) {
        record.seq_no = self.next_sequence_id();

        if self.pending_entries.len() > 50_000 {
            self.pending_entries.drain(10_000..15_000);
            self.add_record(MetricsRecord {
                seq_no: 0,
                timestamp: record.timestamp,
                uptime: record.uptime,
                report_type: obfstr!("metrics-dropped").to_string(),
                payload: format!("count:{}", 5_000),
            });
        }

        self.pending_entries.push_back(record);
    }

    pub fn dequeue_for_report(&mut self) -> Option<VecDeque<MetricsRecord>> {
        const REPORT_MAX_RECORDS: usize = 100;
        if self.pending_entries.len() == 0 {
            return None;
        }

        let entries = if self.pending_entries.len() > REPORT_MAX_RECORDS {
            let pending = self.pending_entries.split_off(REPORT_MAX_RECORDS);
            core::mem::replace(&mut self.pending_entries, pending)
        } else {
            core::mem::replace(&mut self.pending_entries, Default::default())
        };
        Some(entries)
    }

    pub fn enqueue_failed(
        &mut self,
        failed_reports: impl DoubleEndedIterator<Item = MetricsRecord>,
    ) {
        for entry in failed_reports.rev() {
            self.pending_entries.push_front(entry);
        }
    }
}
