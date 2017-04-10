extern crate zipkin_core;

pub use zipkin_core::constants::*;
pub use zipkin_core::{TraceId, SpanId, Timestamp, Endpoint, Annotation, Value, BinaryAnnotation,
                      Annotatable, Span, Sampler, FixedRate, RateLimit, Tracer, Transport,
                      Collector, BaseCollector};

pub mod prelude {
    pub use zipkin_core::{Annotatable, BinaryAnnotationValue};
}

// hack for #[macro_reexport] feature
//
// https://github.com/rust-lang/rust/issues/29638
include!("../../zipkin-core/src/macros.rs");

#[cfg(any(feature = "async", feature = "doc"))]
extern crate zipkin_async;
#[cfg(any(feature = "async", feature = "doc"))]
pub mod async {
    pub use zipkin_async::{AsyncCollector, BaseAsyncCollector};
}

#[cfg(any(feature = "json", feature = "doc"))]
extern crate zipkin_json;
#[cfg(any(feature = "json", feature = "doc"))]
pub mod json {
    pub use zipkin_json::{to_json, to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer,
                          to_writer_pretty};
}

#[cfg(any(feature = "thrift", feature = "doc"))]
extern crate zipkin_thrift;
#[cfg(any(feature = "thrift", feature = "doc"))]
pub mod thrift {
    pub use zipkin_thrift::{to_thrift, to_vec, to_writer, ThriftCodec};
}

pub mod codec {
    #[cfg(any(feature = "json", feature = "doc"))]
    pub use zipkin_json::JsonCodec;
    #[cfg(any(feature = "json", feature = "doc"))]
    pub fn json<T>() -> JsonCodec<T> {
        JsonCodec::new()
    }
    #[cfg(any(feature = "json", feature = "doc"))]
    pub fn pretty_json<T>() -> JsonCodec<T> {
        JsonCodec::pretty()
    }

    #[cfg(any(feature = "thrift", feature = "doc"))]
    pub use zipkin_thrift::ThriftCodec;

    #[cfg(any(feature = "thrift", feature = "doc"))]
    pub fn thrift<T>() -> ThriftCodec<T> {
        ThriftCodec::new()
    }
}

#[cfg(any(feature = "kafka", feature = "doc"))]
extern crate zipkin_kafka;
#[cfg(any(feature = "kafka", feature = "doc"))]
pub mod kafka {
    pub use zipkin_kafka::{KafkaConfig, KafkaTransport};
}

#[cfg(any(feature = "http", feature = "doc"))]
extern crate zipkin_http;
#[cfg(any(feature = "http", feature = "doc"))]
pub mod http {
    pub use zipkin_http::{HttpConfig, HttpTransport};
}
