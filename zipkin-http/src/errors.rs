error_chain! {
    foreign_links {
        IoError(::std::io::Error);
        HttpError(::hyper::Error);
    }
    errors {
        ResponseError(status: ::hyper::status::StatusCode)
    }
}