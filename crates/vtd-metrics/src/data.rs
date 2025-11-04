use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsReport<'a> {
    /// Unique session id for this sender instance.
    pub session_id: &'a str,

    /// Device info
    pub device_info: &'a DeviceInfo,

    /// Entries for the report
    pub records: &'a [MetricsRecord],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsRecord {
    // Entry sequence number
    pub seq_no: u32,

    /// timestamp is a count of 100-nanosecond intervals since January 1, 1601
    pub timestamp: u64,

    /// PCs uptime in counts of 100-nanoseconds
    pub uptime: u64,

    /// Identifyer for the type of report
    pub report_type: String,

    /// User generated payload.
    pub payload: String,
}

#[allow(unused)]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum DeviceInfo {
    Win32(Win32DeviceInfo),
    Unix(UnixDeviceInfo),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnixDeviceInfo {
    pub bios_uuid: Option<String>,
    pub uname: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Win32DeviceInfo {
    pub bios_uuid: Option<String>,

    pub win_major_version: u32,
    pub win_minor_version: u32,
    pub win_build_no: u32,
    pub win_platform_id: u32,
    pub win_unique_build_number: u32,

    pub win_service_pack_major: u16,
    pub win_service_pack_minor: u16,

    pub win_suite_mask: u16,
    pub win_product_type: u8,
}

pub type RequestPostReport<'a> = MetricsReport<'a>;

#[derive(Debug, Deserialize)]
#[serde(tag = "status")]
pub enum ResponsePostReport {
    #[serde(rename = "success")]
    Success,

    #[serde(rename_all = "camelCase")]
    #[serde(rename = "rate-limited")]
    RateLimited {
        /// Retry delay in seconds
        retry_delay: u32,

        /// Sequence numbers of successfully submitted records
        records_submitted: Vec<u32>,
    },

    #[serde(rename_all = "camelCase")]
    #[serde(rename = "generic-error")]
    GenericError { drop_records: bool },

    #[serde(rename_all = "camelCase")]
    #[serde(rename = "instance-blocked")]
    InstanceBlocked,
}
