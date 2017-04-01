extern crate zipkin_core;

pub use zipkin_core::*;

// hack for #[macro_reexport] feature
//
// https://github.com/rust-lang/rust/issues/29638
include!("../../zipkin-core/src/macros.rs");

#[cfg(feature = "async")]
extern crate zipkin_async;
#[cfg(feature = "async")]
pub mod async {
    pub use zipkin_async::*;
}

#[cfg(feature = "json")]
extern crate zipkin_json;
#[cfg(feature = "json")]
pub mod json {
    pub use zipkin_json::*;
}

#[cfg(feature = "thrift")]
extern crate zipkin_thrift;
#[cfg(feature = "thrift")]
pub mod thrift {
    pub use zipkin_thrift::*;
}

#[cfg(feature = "kafka")]
extern crate zipkin_kafka;
#[cfg(feature = "kafka")]
pub mod kafka {
    pub use zipkin_kafka::*;
}

#[cfg(feature = "http")]
extern crate zipkin_http;
#[cfg(feature = "http")]
pub mod http {
    pub use zipkin_http::*;
}

pub mod codec {
    #[cfg(feature = "json")]
    pub fn json<T>() -> ::zipkin_json::JsonCodec<T> {
        ::zipkin_json::JsonCodec::new()
    }

    #[cfg(feature = "json")]
    pub fn pretty_json<T>() -> ::zipkin_json::JsonCodec<T> {
        ::zipkin_json::JsonCodec::pretty()
    }

    #[cfg(feature = "thrift")]
    pub fn thrift<T>() -> ::zipkin_thrift::ThriftCodec<T> {
        ::zipkin_thrift::ThriftCodec::new()
    }
}