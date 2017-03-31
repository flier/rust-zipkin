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
