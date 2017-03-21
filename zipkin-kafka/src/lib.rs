#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate futures_cpupool;
extern crate kafka;
extern crate zipkin;

mod errors;
mod collector;

pub use collector::{KafkaConfig, KafkaCollector};