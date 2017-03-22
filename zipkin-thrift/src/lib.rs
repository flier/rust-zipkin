#[macro_use]
extern crate error_chain;
extern crate byteorder;
extern crate ordered_float;
extern crate thrift;
extern crate try_from;
extern crate zipkin;

#[cfg(test)]
extern crate chrono;

mod core;
mod errors;
mod encode;

pub use encode::{to_thrift, to_vec, to_writer};

#[cfg(feature = "future")]
extern crate tokio_io;
#[cfg(feature = "future")]
extern crate bytes;

#[cfg(feature = "future")]
mod codec;

#[cfg(feature = "future")]
pub use codec::ThriftCodec;
