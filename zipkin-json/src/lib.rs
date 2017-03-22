#[macro_use]
extern crate error_chain;
extern crate serde_json;
extern crate base64;
extern crate zipkin;

#[cfg(test)]
extern crate chrono;
#[cfg(test)]
extern crate diff;

mod errors;
mod encode;

pub use encode::{to_json, to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer,
                 to_writer_pretty};


#[cfg(feature = "future")]
extern crate tokio_io;
#[cfg(feature = "future")]
extern crate bytes;

#[cfg(feature = "future")]
mod codec;

#[cfg(feature = "future")]
pub use codec::JsonCodec;