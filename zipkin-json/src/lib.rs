extern crate serde_json;
extern crate zipkin;

mod encode;

pub use encode::{to_json, to_string, to_vec, to_writer};