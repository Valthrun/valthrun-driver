#![feature(deadline_api)]

mod client;
mod crypto;
mod data;
mod device;
mod error;
mod queue;
mod sender;

const KEY_METRICS_INTERNAL: &str = "metrics-internal";

pub use client::{
    MetricsClient,
    create_instance,
};
