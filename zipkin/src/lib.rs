#![recursion_limit = "65536"]

#[macro_use]
extern crate error_chain;

extern crate zipkin_core;

pub mod errors;
pub use errors::{Error, ErrorKind, Result};

pub mod core {
    pub use zipkin_core::*;
    pub use zipkin_core::errors::*;
}

pub use core::constants::*;
pub use core::{TraceId, SpanId, Timestamp, Endpoint, Annotation, Value, BinaryAnnotation,
               Annotatable, Span, FixedRate, RateLimit, Tracer, MimeType};

pub trait Codec<'a>: core::Codec<Item = Vec<Span<'a>>, Error = Error> + MimeType {}

impl<'a, T> Codec<'a> for T where T: core::Codec<Item = Vec<Span<'a>>, Error = Error> + MimeType {}

pub type BoxCodec<'a> = Box<Codec<'a, Item = Vec<Span<'a>>, Error = Error>>;

pub trait Sampler<'a>: core::Sampler<Item = Span<'a>> {}

impl<'a, T> Sampler<'a> for T where T: core::Sampler<Item = core::Span<'a>> {}

pub trait Transport<B>: core::Transport<Buffer = B, Output = (), Error = Error>
    where B: AsRef<[u8]> + Send
{
}

impl<B, T> Transport<B> for T
    where B: AsRef<[u8]> + Send,
          T: core::Transport<Buffer = B, Output = (), Error = Error>
{
}

pub type BoxTransport<B> = Box<core::Transport<Buffer = B, Output = (), Error = Error>>;

pub trait Collector<'a>
    : core::Collector<Item = Vec<Span<'a>>, Output = (), Error = Error> {
}

impl<'a, T> Collector<'a> for T
    where T: core::Collector<Item = Vec<Span<'a>>, Output = (), Error = Error>
{
}

pub type BoxCollector<'a> = Box<Collector<'a, Item = Vec<Span<'a>>, Output = (), Error = Error>>;

pub type BaseCollector<'a, C, T> = core::BaseCollector<'a, C, T, Error>;

pub mod prelude {
    pub use core::{Annotatable, BinaryAnnotationValue, MimeType};
}

pub mod collector {
    use super::{Codec, Transport, BaseCollector};
    use super::core::BytesMut;

    pub fn new<'a, C, T>(codec: Box<C>, transport: Box<T>) -> BaseCollector<'a, C, T>
        where C: Codec<'a> + ?Sized,
              T: Transport<BytesMut> + ?Sized
    {
        BaseCollector::new(codec, transport)
    }
}

// hack for #[macro_reexport] feature
//
// https://github.com/rust-lang/rust/issues/29638
include!("../../zipkin-core/src/macros.rs");

#[cfg(any(feature = "async", feature = "doc"))]
extern crate zipkin_async;
#[cfg(any(feature = "async", feature = "doc"))]
pub mod async {
    pub use zipkin_async::errors::{Error, ErrorKind, Result};
    pub use zipkin_async::{AsyncCollector, BaseAsyncCollector};
}

#[cfg(any(feature = "json", feature = "doc"))]
extern crate zipkin_json;
#[cfg(any(feature = "json", feature = "doc"))]
pub mod json {
    pub use zipkin_json::errors::{Error, ErrorKind, Result};
    pub use zipkin_json::{to_json, to_string, to_string_pretty, to_vec, to_vec_pretty, to_writer,
                          to_writer_pretty, JsonCodec as Codec};
}

#[cfg(any(feature = "thrift", feature = "doc"))]
extern crate zipkin_thrift;
#[cfg(any(feature = "thrift", feature = "doc"))]
pub mod thrift {
    pub use zipkin_thrift::errors::{Error, ErrorKind, Result};
    pub use zipkin_thrift::{to_thrift, to_vec, to_writer, ThriftCodec as Codec};
}

pub mod codec {
    use super::{Span, Error};

    #[cfg(any(feature = "json", feature = "doc"))]
    pub fn json<'a>() -> super::json::Codec<Vec<Span<'a>>, Error> {
        super::json::Codec::new()
    }

    #[cfg(any(feature = "json", feature = "doc"))]
    pub fn pretty_json<'a>() -> super::json::Codec<Vec<Span<'a>>, Error> {
        super::json::Codec::pretty()
    }

    #[cfg(any(feature = "thrift", feature = "doc"))]
    pub fn thrift<'a>() -> super::thrift::Codec<Vec<Span<'a>>, Error> {
        super::thrift::Codec::new()
    }
}

#[cfg(any(feature = "kafka", feature = "doc"))]
extern crate zipkin_kafka;
#[cfg(any(feature = "kafka", feature = "doc"))]
pub mod kafka {
    pub use zipkin_kafka::errors::{Error, ErrorKind, Result};
    pub use zipkin_kafka::{KafkaConfig as Config, KafkaTransport as Transport};
}

#[cfg(any(feature = "http", feature = "doc"))]
extern crate zipkin_http;
#[cfg(any(feature = "http", feature = "doc"))]
pub mod http {
    pub use zipkin_http::errors::{Error, ErrorKind, Result};
    pub use zipkin_http::{HttpConfig as Config, HttpTransport as Transport};
}
