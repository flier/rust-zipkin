#[macro_use]
extern crate error_chain;
extern crate hyper;
extern crate zipkin;

pub mod errors;
mod transport;

pub use transport::{HttpConfig, HttpTransport};