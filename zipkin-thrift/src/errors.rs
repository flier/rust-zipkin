error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Thrift(::thrift::Error);
    }
}