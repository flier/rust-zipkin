error_chain! {
    foreign_links {
        IoError(::std::io::Error);
        SystemTimeErro(::std::time::SystemTimeError);
    }
    errors {
        SendError
        PoisonError {
            description("poisoned lock: another task failed inside")
        }
    }
}

impl<T> From<::std::sync::PoisonError<T>> for Error {
    fn from(err: ::std::sync::PoisonError<T>) -> Self {
        ErrorKind::PoisonError.into()
    }
}