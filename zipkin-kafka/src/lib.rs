#[macro_use]
extern crate error_chain;
extern crate kafka;

extern crate zipkin_core;

pub use kafka::producer::{Compression, RequiredAcks};
pub use kafka::error::Error as KafkaError;

pub mod errors;
mod transport;

pub use transport::{KafkaConfig, KafkaTransport};
