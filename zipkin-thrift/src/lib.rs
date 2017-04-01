#[macro_use]
extern crate error_chain;
extern crate byteorder;
extern crate ordered_float;
extern crate thrift;
extern crate try_from;
extern crate bytes;
extern crate tokio_io;

extern crate zipkin_core;

#[cfg(test)]
extern crate chrono;

mod core;
pub mod errors;
mod encode;
mod codec;

pub use encode::{to_thrift, to_vec, to_writer};
pub use codec::ThriftCodec;
