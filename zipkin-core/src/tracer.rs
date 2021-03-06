use sampler::Sampler;
use span::{Span, now};
use collector::Collector;

#[derive(Clone, Debug, Default)]
pub struct Tracer<S, C: ?Sized> {
    pub sampler: Option<S>,
    pub collector: Box<C>,
}

impl<'a, S, C: ?Sized> Tracer<S, C> {
    pub fn new(collector: Box<C>) -> Self {
        Tracer {
            sampler: None,
            collector: collector,
        }
    }

    pub fn with_sampler(sampler: S, collector: Box<C>) -> Self {
        Tracer {
            sampler: Some(sampler),
            collector: collector,
        }
    }
}

impl<'a, S, C> Tracer<S, C>
    where S: Sampler<Item = Span<'a>>,
          C: ?Sized
{
    pub fn span(&self, name: &'a str) -> Span<'a> {
        let span = Span::new(name);
        let sampled = self.sampler
            .as_ref()
            .map(|sampler| sampler.sample(&span));

        Span {
            sampled: sampled,
            ..span
        }
    }
}

impl<'a, S, C> Tracer<S, C>
    where C: Collector<Item = Vec<Span<'a>>> + ?Sized
{
    pub fn submit(&self,
                  mut span: Span<'a>)
                  -> Result<<C as Collector>::Output, <C as Collector>::Error> {
        span.duration = Some(now() - span.timestamp);

        self.collector.submit(vec![span])
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use super::super::*;
    use super::*;
    use errors::*;

    struct MockCollector<'a, T: 'a>(PhantomData<&'a T>);

    unsafe impl<'a, T> Sync for MockCollector<'a, T> {}
    unsafe impl<'a, T> Send for MockCollector<'a, T> {}

    impl<'a> Default for MockCollector<'a, Span<'a>> {
        fn default() -> Self {
            MockCollector(PhantomData)
        }
    }

    impl<'a> Collector for MockCollector<'a, Span<'a>> {
        type Item = Span<'a>;
        type Output = ();
        type Error = Error;

        fn submit(&self, _: Span<'a>) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn sampling() {
        let mut tracer = Tracer::with_sampler(FixedRate::new(2),
                                              Box::new(MockCollector::default()));

        assert_eq!(tracer.span("test1").sampled, Some(true));
        assert_eq!(tracer.span("test2").sampled, Some(false));
        assert_eq!(tracer.span("test3").sampled, Some(true));

        tracer = Tracer::new(Box::new(MockCollector::default()));

        assert_eq!(tracer.span("test1").sampled, None);
    }
}