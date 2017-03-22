#[macro_use]
extern crate error_chain;
extern crate tokio_io;
extern crate bytes;
extern crate hyper;
extern crate zipkin;

pub mod errors;
mod collector;

pub use collector::{HttpConfig, HttpCollector};