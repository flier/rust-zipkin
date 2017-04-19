error_chain!{
    foreign_links {
        IoError(::std::io::Error);
    }

    errors {
        LockError
    }
}

unsafe impl Sync for Error {}
unsafe impl Send for Error {}

impl<T> From<::std::sync::PoisonError<T>> for Error {
    fn from(_: ::std::sync::PoisonError<T>) -> Self {
        ErrorKind::LockError.into()
    }
}