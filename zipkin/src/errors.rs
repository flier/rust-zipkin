error_chain!{
    foreign_links {
        IoError(::std::io::Error);
        JsonError(::zipkin_json::JsonError) #[cfg(any(feature = "json", feature = "doc"))];
        ThriftError(::zipkin_thrift::ThriftError) #[cfg(any(feature = "thrift", feature = "doc"))];
        KafkaError(::zipkin_kafka::KafkaError) #[cfg(any(feature = "kafka", feature = "doc"))];
        HttpError(::zipkin_http::HttpError) #[cfg(any(feature = "http", feature = "doc"))];
    }
    links {
        Async(::zipkin_async::errors::Error, ::zipkin_async::errors::ErrorKind) #[cfg(any(feature = "async", feature = "doc"))];
        Json(::zipkin_json::errors::Error, ::zipkin_json::errors::ErrorKind) #[cfg(any(feature = "json", feature = "doc"))];
        Thrift(::zipkin_thrift::errors::Error, ::zipkin_thrift::errors::ErrorKind) #[cfg(any(feature = "thrift", feature = "doc"))];
        Kafka(::zipkin_kafka::errors::Error, ::zipkin_kafka::errors::ErrorKind) #[cfg(any(feature = "kafka", feature = "doc"))];
        Http(::zipkin_http::errors::Error, ::zipkin_http::errors::ErrorKind) #[cfg(any(feature = "http", feature = "doc"))];
    }
    errors {
        UnknownCodec(name: String)
    }
}

unsafe impl Sync for Error {}
unsafe impl Send for Error {}