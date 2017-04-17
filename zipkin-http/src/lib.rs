#[macro_use]
extern crate error_chain;
extern crate hyper;

extern crate zipkin_core;

pub use hyper::client::RedirectPolicy;
pub use hyper::Error as HttpError;

pub mod errors;
mod transport;

pub use transport::{HttpConfig, HttpTransport};
