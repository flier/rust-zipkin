use futures::Future;

use span::Span;

pub trait Collector<'a> {
    type Error;

    fn submit(&self, span: Span<'a>) -> Box<Future<Item = Span<'a>, Error = Self::Error>>;
}