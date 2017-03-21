error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Kafka(::kafka::error::Error);
    }
}