error_chain! {
    foreign_links {
        IoError(::std::io::Error);
        HttpError(::hyper::Error);
        UrlError(::hyper::error::ParseError);
    }
    errors {
        ResponseError(status: ::hyper::status::StatusCode)
    }
}