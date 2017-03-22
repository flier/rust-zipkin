error_chain! {
    foreign_links {
        IoError(::std::io::Error);
        SystemTimeErro(::std::time::SystemTimeError);
    }
    errors {
        SendError
        PoisonError
    }
}

impl<T> From<::std::sync::mpsc::SendError<T>> for Error {
    fn from(_: ::std::sync::mpsc::SendError<T>) -> Self {
        ErrorKind::SendError.into()
    }
}

impl<T> From<::std::sync::PoisonError<T>> for Error {
    fn from(_: ::std::sync::PoisonError<T>) -> Self {
        ErrorKind::PoisonError.into()
    }
}