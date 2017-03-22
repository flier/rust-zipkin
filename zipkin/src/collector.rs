use span::Span;

pub trait Collector<'a> {
    type Error;

    fn submit(&mut self, span: Span<'a>) -> Result<(), Self::Error>;
}
