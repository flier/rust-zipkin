#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate futures_cpupool;
extern crate zipkin;
extern crate bytes;
extern crate tokio_io;

pub mod errors;
mod collector;

pub use collector::{AsyncCollector, BaseAsyncCollector};