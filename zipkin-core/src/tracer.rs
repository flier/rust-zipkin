use sampler::Sampler;
use span::Span;
use collector::Collector;

#[derive(Clone, Debug, Default)]
pub struct Tracer<S, C> {
    pub sampler: Option<S>,
    pub collector: C,
}

impl<'a, S, C> Tracer<S, C> {
    pub fn new(collector: C) -> Self {
        Tracer {
            sampler: None,
            collector: collector,
        }
    }

    pub fn with_sampler(sampler: S, collector: C) -> Self {
        Tracer {
            sampler: Some(sampler),
            collector: collector,
        }
    }
}

impl<'a, S, C> Tracer<S, C>
    where S: Sampler<Item = Span<'a>>,
          C: Collector<Item = Span<'a>>
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

    pub fn submit(&self,
                  span: Span<'a>)
                  -> Result<<C as Collector>::Output, <C as Collector>::Error> {
        self.collector.submit(span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::sampler::*;

    #[test]
    fn sampling() {
        let mut tracer = Tracer::with_sampler(FixedRate::new(2));

        assert_eq!(tracer.span("test1").sampled, Some(true));
        assert_eq!(tracer.span("test2").sampled, Some(false));
        assert_eq!(tracer.span("test3").sampled, Some(true));

        tracer = Tracer::new();

        assert_eq!(tracer.span("test1").sampled, None);
    }
}