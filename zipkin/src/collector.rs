use futures::Future;

use span::Span;

pub trait Collector<'a> {
    type Error;

    fn submit(span: &Span<'a>) -> Future<Item = Span<'a>, Error = Self::Error>;
}