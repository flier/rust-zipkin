use std::str;
use std::sync::Mutex;
use std::marker::PhantomData;

use bytes::BytesMut;

use hexplay::HexViewBuilder;

use tokio_io::codec::Encoder;
use mime::Mime;

use span::Span;

pub trait MimeType {
    fn mime_type(&self) -> Mime;
}

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

pub struct BaseCollector<C, T, E> {
    pub max_message_size: usize,
    pub encoder: Mutex<C>,
    pub transport: Mutex<T>,
    phantom: PhantomData<E>,
}

impl<C, T, E> BaseCollector<C, T, E> {
    pub fn new(encoder: C, transport: T) -> Self {
        BaseCollector {
            max_message_size: 4096,
            encoder: Mutex::new(encoder),
            transport: Mutex::new(transport),
            phantom: PhantomData,
        }
    }
}

impl<'a: 'b, 'b, C, T, E> Collector for BaseCollector<C, T, E>
    where C: Encoder<Item = Vec<Span<'a>>, Error = E> + Sync + Send,
          T: Transport<BytesMut, Error = E>,
          E: From<::std::io::Error> + Sync + Send
{
    type Item = Vec<Span<'a>>;
    type Output = ();
    type Error = E;

    fn submit(&self, spans: Self::Item) -> Result<Self::Output, Self::Error> {
        let mut buf = BytesMut::with_capacity(self.max_message_size);
        {
            if let Ok(mut encoder) = self.encoder.lock() {
                let count = spans.len();

                encoder.encode(spans, &mut buf)?;

                debug!("encoded {} spans:\n{}",
                       count,
                       if buf[0] == b'[' {
                           String::from_utf8(buf.to_vec()).unwrap()
                       } else {
                           HexViewBuilder::new(&buf[..]).finish().to_string()
                       });
            }
        }

        {
            if let Ok(mut transport) = self.transport.lock() {
                transport.send(&buf)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::marker::PhantomData;

    use bytes::{BytesMut, BufMut};

    use tokio_io::codec::Encoder;

    use super::Collector;
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

    impl<'a> Encoder for MockEncoder<'a, Vec<Span<'a>>> {
        type Item = Vec<Span<'a>>;
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

        let collector = BaseCollector {
            max_message_size: 1024,
            encoder: Mutex::new(MockEncoder::new()),
            transport: Mutex::new(MockTransport::new()),
            phantom: PhantomData,
        };

        collector.submit(vec![span]).unwrap();

        assert_eq!(collector.encoder.lock().unwrap().encoded, 1);
        assert_eq!(collector.transport.lock().unwrap().sent, 1);
        assert_eq!(collector.transport.lock().unwrap().buf, b"hello world");
    }
}