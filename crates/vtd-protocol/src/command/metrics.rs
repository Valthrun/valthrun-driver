use core::ptr;

use crate::utils;

#[derive(Debug, Clone, Copy)]
pub struct DriverCommandMetricsReportSend {
    pub report_type: [u8; 0x100],

    pub report_payload: *const u8,
    pub report_payload_length: usize,
}

impl DriverCommandMetricsReportSend {
    pub fn get_report_type(&self) -> Option<&str> {
        utils::fixed_buffer_to_str(&self.report_type)
    }

    pub fn set_report_type(&mut self, value: &str) -> bool {
        utils::str_to_fixed_buffer(&mut self.report_type, value)
    }
}

impl Default for DriverCommandMetricsReportSend {
    fn default() -> Self {
        Self {
            report_type: [0x0; 0x100],

            report_payload: ptr::null_mut(),
            report_payload_length: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DriverCommandMetricsFlush {
    // In:
    pub blocking: bool,

    // Out:
    pub queue_remaining: usize,
}

impl Default for DriverCommandMetricsFlush {
    fn default() -> Self {
        DriverCommandMetricsFlush {
            blocking: true,
            queue_remaining: 0,
        }
    }
}
