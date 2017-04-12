use std::sync::Mutex;

use bytes::BytesMut;

use tokio_io::codec::Encoder;

use span::Span;
use errors::Error;

pub trait Transport<B: AsRef<[u8]>>
    where Self: 'static + Send
{
    type Output: Send;
    type Error;

    fn send(&mut self, buf: &B) -> Result<Self::Output, Self::Error>;
}

pub trait Collector: Sync + Send {
    type Item;
    type Output: Send;
    type Error;

    fn submit(&self, item: Self::Item) -> Result<Self::Output, Self::Error>;
}

pub struct BaseCollector<E, T> {
    pub max_message_size: usize,
    pub encoder: Mutex<E>,
    pub transport: Mutex<T>,
}

impl<'a, E, T> Collector for BaseCollector<E, T>
    where E: Encoder<Item = Span<'a>, Error = Error> + Sync + Send,
          T: Transport<BytesMut, Error = Error>
{
    type Item = Span<'a>;
    type Output = ();
    type Error = Error;

    fn submit(&self, span: Span<'a>) -> Result<Self::Output, Self::Error> {
        let mut buf = BytesMut::with_capacity(self.max_message_size);
        {
            let mut encoder = self.encoder.lock()?;

            encoder.encode(span, &mut buf)?
        }

        {
            let mut transport = self.transport.lock()?;

            transport.send(&buf)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use bytes::{BytesMut, BufMut};

    use tokio_io::codec::Encoder;

    use super::super::*;
    use super::super::errors::Error;

    struct MockTransport {
        sent: usize,
        buf: Vec<u8>,
    }

    impl MockTransport {
        fn new() -> Self {
            MockTransport {
                sent: 0,
                buf: vec![],
            }
        }
    }

    impl<B: AsRef<[u8]>> Transport<B> for MockTransport {
        type Output = ();
        type Error = Error;

        fn send(&mut self, buf: &B) -> Result<Self::Output, Self::Error> {
            self.sent += 1;
            self.buf.append(&mut buf.as_ref().to_vec());

            Ok(())
        }
    }

    struct MockEncoder<'a, T: 'a> {
        encoded: usize,
        phantom: PhantomData<&'a T>,
    }

    impl<'a, T> MockEncoder<'a, T> {
        fn new() -> Self {
            MockEncoder {
                encoded: 0,
                phantom: PhantomData,
            }
        }
    }

    impl<'a> Encoder for MockEncoder<'a, Span<'a>> {
        type Item = Span<'a>;
        type Error = Error;

        fn encode(&mut self, _: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
            self.encoded += 1;

            buf.put("hello");
            buf.put(" world");

            Ok(())
        }
    }

    #[test]
    fn submit() {
        let span = Span::new("test");

        let mut collector = BaseCollector {
            max_message_size: 1024,
            encoder: MockEncoder::new(),
            transport: MockTransport::new(),
        };

        collector.submit(span).unwrap();

        assert_eq!(collector.encoder.encoded, 1);
        assert_eq!(collector.transport.sent, 1);
        assert_eq!(collector.transport.buf, b"hello world");
    }
}