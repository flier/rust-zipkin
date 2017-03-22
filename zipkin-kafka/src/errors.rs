error_chain! {
    foreign_links {
        IoError(::std::io::Error);
        KafkaError(::kafka::error::Error);
    }
}