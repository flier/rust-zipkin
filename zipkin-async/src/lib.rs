#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate futures_cpupool;
extern crate bytes;
extern crate tokio_io;

extern crate zipkin_core;

pub mod errors;
mod collector;

pub use collector::{AsyncCollector, BaseAsyncCollector};