#![feature(deadline_api)]

mod client;
mod crypto;
mod data;
mod device;
mod error;
mod queue;
mod sender;

const KEY_METRICS_INTERNAL: &str = "metrics-internal";
pub const MK_APPLICATION_TYPE: &str = "application-type";
pub const MK_INTERFACE_TYPE: &str = "interface-type";

pub use client::{
    MetricsClient,
    create_instance,
};
