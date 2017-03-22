#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate tokio_io;
extern crate bytes;
extern crate kafka;
extern crate zipkin;

pub mod errors;
mod collector;

pub use collector::{KafkaConfig, KafkaCollector};