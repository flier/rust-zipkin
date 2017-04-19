#[macro_use]
extern crate error_chain;
extern crate serde_json;
extern crate base64;
extern crate bytes;
#[macro_use]
extern crate mime;

extern crate zipkin_core;

#[cfg(test)]
extern crate diff;

pub use serde_json::Error as JsonError;

pub mod errors;
mod encode;
mod codec;

pub use encode::{ToJson, to_json, to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer,
                 to_writer_pretty};
pub use codec::JsonCodec;