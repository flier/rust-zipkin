error_chain! {
    foreign_links {
        IoError(::std::io::Error);
        JsonError(::serde_json::error::Error);
    }
}