extern crate serde_json;
extern crate zipkin;

#[cfg(test)]
extern crate chrono;
#[cfg(test)]
extern crate diff;

mod encode;

pub use encode::{to_json, to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer,
                 to_writer_pretty};