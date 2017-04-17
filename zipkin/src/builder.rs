use std::str::FromStr;

use mime::Mime;
use url::Url;

use zipkin_core::MimeType;
#[cfg(any(feature = "json", feature = "doc"))]
use zipkin_json::JsonCodec;
#[cfg(any(feature = "thrift", feature = "doc"))]
use zipkin_thrift::ThriftCodec;

use errors::ErrorKind;

pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 4096;

pub enum MessageEncoder<T, E> {
    #[cfg(any(feature = "json", feature = "doc"))]
    Json(JsonCodec<T, E>),
    #[cfg(any(feature = "json", feature = "doc"))]
    PrettyJson(JsonCodec<T, E>),
    #[cfg(any(feature = "thrift", feature = "doc"))]
    Thrift(ThriftCodec<T, E>),
}

impl<T, E> MessageEncoder<T, E> {
    pub fn mime_type(&self) -> Mime {
        match self {
            &MessageEncoder::Json(ref codec) |
            &MessageEncoder::PrettyJson(ref codec) => codec.mime_type(),
            &MessageEncoder::Thrift(ref codec) => codec.mime_type(),
        }
    }
}

impl<T, E> FromStr for MessageEncoder<T, E>
    where E: From<ErrorKind>
{
    type Err = E;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(any(feature = "json", feature = "doc"))]
            "json" => Ok(MessageEncoder::Json(JsonCodec::new())),
            #[cfg(any(feature = "json", feature = "doc"))]
            "pretty" | "pretty-json" => Ok(MessageEncoder::PrettyJson(JsonCodec::pretty())),
            #[cfg(any(feature = "thrift", feature = "doc"))]
            "thrift" => Ok(MessageEncoder::Thrift(ThriftCodec::new())),
            _ => bail!(ErrorKind::UnknownCodec(s.to_owned())),
        }
    }
}

pub struct CollectorBuilder<T, E> {
    max_message_size: usize,
    encoder: MessageEncoder<T, E>,
}

impl<T, E> CollectorBuilder<T, E> {
    fn new(encoder: MessageEncoder<T, E>) -> Self {
        CollectorBuilder {
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
            encoder: encoder,
        }
    }

    #[cfg(any(feature = "json", feature = "doc"))]
    pub fn json() -> Self {
        CollectorBuilder::new(MessageEncoder::Json(JsonCodec::new()))
    }

    #[cfg(any(feature = "json", feature = "doc"))]
    pub fn pretty_json() -> Self {
        CollectorBuilder::new(MessageEncoder::Json(JsonCodec::pretty()))
    }

    #[cfg(any(feature = "thrift", feature = "doc"))]
    pub fn thrift() -> Self {
        CollectorBuilder::new(MessageEncoder::Thrift(ThriftCodec::new()))
    }

    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }

    pub fn with_max_message_size(&mut self, max_message_size: usize) -> &mut Self {
        self.max_message_size = max_message_size;
        self
    }

    #[cfg(any(feature = "kafka", feature = "doc"))]
    pub fn with_kafka(self, hosts: &[String], topic: &str) -> kafka::Builder<T, E> {
        kafka::Builder::new(self, hosts, topic)
    }

    #[cfg(any(feature = "http", feature = "doc"))]
    pub fn with_http(self, url: Url) -> http::Result<http::Builder<T, E>> {
        http::Builder::new(self, url)
    }
}

#[cfg(any(feature = "kafka", feature = "doc"))]
pub mod kafka {
    use std::time::Duration;

    use zipkin_kafka::{KafkaConfig, Compression, RequiredAcks};
    pub use zipkin_kafka::errors::Result;

    use super::CollectorBuilder;

    pub struct Builder<T, E> {
        builder: CollectorBuilder<T, E>,
        config: KafkaConfig,
    }

    impl<T, E> Builder<T, E> {
        pub fn new(builder: CollectorBuilder<T, E>, hosts: &[String], topic: &str) -> Self {
            Builder {
                builder: builder,
                config: KafkaConfig {
                    hosts: hosts.to_vec(),
                    topic: topic.to_owned(),
                    ..Default::default()
                },
            }
        }

        pub fn with_compression(&mut self, compression: Compression) -> &mut Self {
            self.config.compression = compression;
            self
        }
        pub fn with_ack_timeout(&mut self, ack_timeout: Duration) -> &mut Self {
            self.config.ack_timeout = ack_timeout;
            self
        }
        pub fn with_connection_idle_timeout(&mut self,
                                            connection_idle_timeout: Duration)
                                            -> &mut Self {
            self.config.connection_idle_timeout = connection_idle_timeout;
            self
        }
        pub fn with_required_acks(&mut self, required_acks: RequiredAcks) -> &mut Self {
            self.config.required_acks = required_acks;
            self
        }
    }
}

#[cfg(any(feature = "http", feature = "doc"))]
pub mod http {
    use std::time::Duration;

    use url::Url;

    use zipkin_http::{HttpConfig, RedirectPolicy};
    pub use zipkin_http::errors::Result;

    use super::CollectorBuilder;

    pub struct Builder<T, E> {
        builder: CollectorBuilder<T, E>,
        url: Url,
        config: HttpConfig,
    }

    impl<T, E> Builder<T, E> {
        pub fn new(builder: CollectorBuilder<T, E>, url: Url) -> Result<Self> {
            let mime_type = builder.encoder.mime_type();

            Ok(Builder {
                   builder: builder,
                   url: url,
                   config: HttpConfig::new(mime_type),
               })
        }

        pub fn with_redirect_policy(&mut self, redirect_policy: RedirectPolicy) -> &mut Self {
            self.config.redirect_policy = redirect_policy;
            self
        }
        pub fn with_read_timeout(&mut self, read_timeout: Duration) -> &mut Self {
            self.config.read_timeout = Some(read_timeout);
            self
        }
        pub fn with_write_timeout(&mut self, write_timeout: Duration) -> &mut Self {
            self.config.write_timeout = Some(write_timeout);
            self
        }
        pub fn with_max_idle_connections(&mut self, max_idle_connections: usize) -> &mut Self {
            self.config.max_idle_connections = Some(max_idle_connections);
            self
        }
    }
}