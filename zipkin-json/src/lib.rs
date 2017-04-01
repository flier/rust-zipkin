#[macro_use]
extern crate error_chain;
extern crate serde_json;
extern crate base64;
extern crate bytes;
extern crate tokio_io;
extern crate zipkin;

#[cfg(test)]
extern crate chrono;
#[cfg(test)]
extern crate diff;

mod errors;
mod encode;
mod codec;

pub use encode::{to_json, to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer,
                 to_writer_pretty};
pub use codec::JsonCodec;