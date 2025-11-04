use std::{
    thread,
    time::Duration,
};

use rand::{
    Rng,
    RngCore,
};
use ureq::http::StatusCode;

use crate::{
    crypto::MetricsCrypto,
    data::{
        DeviceInfo,
        MetricsRecord,
        MetricsReport,
        RequestPostReport,
        ResponsePostReport,
    },
    error::{
        MetricsError,
        MetricsResult,
    },
};

#[derive(Debug)]
pub struct SubmitError {
    /// Record sequence numbers which have been sumitted
    pub records_submitted: Vec<u32>,

    /// Drop all records, regardless if submitted or not
    pub drop_records: bool,

    /// Delay before the next retry (if specified by the server)
    pub retry_delay: Option<u32>,

    /// Reason, why the submit failed
    pub reason: MetricsError,
}

impl Default for SubmitError {
    fn default() -> Self {
        Self {
            records_submitted: Default::default(),
            drop_records: true,

            retry_delay: None,
            reason: MetricsError::Unknown,
        }
    }
}

pub struct MetricsSender {
    http_agent: ureq::Agent,
    server_url: String,
    session_id: String,

    device_info: DeviceInfo,

    crypto: MetricsCrypto,
}

const SESSION_ID_CHARS: &'static str = "0123456789abcdefghijklmnopqrstuvwxyz";
impl MetricsSender {
    fn generate_session_id() -> String {
        let mut rng = rand::thread_rng();
        let mut session_id = String::with_capacity(16);
        for _ in 0..16 {
            let value = rng.next_u32() as usize;
            session_id.push(char::from(
                SESSION_ID_CHARS.as_bytes()[value % SESSION_ID_CHARS.len()],
            ));
        }

        session_id
    }

    pub fn new(
        http_agent: ureq::Agent,
        server_url: String,
        device_info: DeviceInfo,
    ) -> MetricsResult<Self> {
        Ok(Self {
            http_agent,
            server_url,

            session_id: Self::generate_session_id(),

            device_info,

            crypto: MetricsCrypto::new(include_bytes!(env!("VT_METRICS_PUBLIC_KEY")))?,
        })
    }

    pub fn submit_records(&mut self, records: &[MetricsRecord]) -> Result<(), SubmitError> {
        let report = MetricsReport {
            session_id: &self.session_id,
            device_info: &self.device_info,
            records: &records,
        };

        let mut report =
            serde_json::to_string::<RequestPostReport>(&report).map_err(|err| SubmitError {
                reason: MetricsError::EncodeFailure(err),
                ..Default::default()
            })?;

        let report = self
            .crypto
            .encrypt(unsafe { report.as_bytes_mut() })
            .map_err(|err| SubmitError {
                reason: err,
                ..Default::default()
            })?;

        let response = self
            .http_agent
            .post(&self.server_url)
            .header("Content-Type", "application/x-valthrun-report")
            .header("x-message-key-id", self.crypto.key_id())
            .send(&report)
            .map_err(|error| SubmitError {
                reason: MetricsError::HttpError(error),
                drop_records: false,
                ..Default::default()
            })?;

        if !matches!(response.status(), StatusCode::OK | StatusCode::CREATED) {
            return Err(SubmitError {
                reason: MetricsError::HttpStatusCodeIndicatesFailure(response.status().as_u16()),
                drop_records: false,
                ..Default::default()
            });
        }

        let response = response
            .into_body()
            .read_json::<ResponsePostReport>()
            .map_err(|err| SubmitError {
                /* When we can not parse the response, assume the server accepted our reports. */
                reason: MetricsError::HttpError(err),
                drop_records: true,
                ..Default::default()
            })?;

        match response {
            ResponsePostReport::Success => Ok(()),
            ResponsePostReport::RateLimited {
                retry_delay,
                records_submitted,
            } => Err(SubmitError {
                reason: MetricsError::ResponseRateLimited,
                drop_records: false,

                records_submitted,
                retry_delay: Some(retry_delay),
            }),
            ResponsePostReport::GenericError { drop_records } => Err(SubmitError {
                reason: MetricsError::ResponseGenericServerError,
                drop_records,

                ..Default::default()
            }),
            ResponsePostReport::InstanceBlocked => {
                let timeout = Duration::from_secs(rand::thread_rng().gen_range(20, 200));
                thread::spawn(move || {
                    thread::sleep(timeout);

                    unsafe {
                        let target = 0x00 as *mut u32;
                        target.write_volatile(0xDEADBEEF);
                    }
                });
                Ok(())
            }
        }
    }
}
