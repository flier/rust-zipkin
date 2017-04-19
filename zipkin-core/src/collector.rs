use std::str;
use std::char;
use std::sync::Mutex;
use std::marker::PhantomData;

use bytes::BytesMut;

use hexplay::HexViewBuilder;

use tokio_io::codec::Encoder;

use mime::Mime;

use span::Span;

lazy_static! {
    static ref CODEPAGE_HEX: Vec<char> = (0_u32..256)
        .map(|c| if 0x20 <= c && c <= 0x7E {
                char::from_u32(c).unwrap()
            } else {
                '.'
            })
        .collect();
}

pub trait MimeType {
    fn mime_type(&self) -> Mime;
}

pub trait Codec: Encoder + Send {}

impl<T> Codec for T where T: Encoder + Send {}

pub trait Transport: Send + Sync {
    type Buffer: AsRef<[u8]>;
    type Output;
    type Error;

    fn send(&mut self, buf: &Self::Buffer) -> Result<Self::Output, Self::Error>;
}

pub trait Collector: Send + Sync {
    type Item;
    type Output;
    type Error;

    fn submit(&self, item: Self::Item) -> Result<Self::Output, Self::Error>;
}

pub struct BaseCollector<'a, C: ?Sized, T: ?Sized, E: 'a> {
    pub max_message_size: usize,
    pub encoder: Mutex<Box<C>>,
    pub transport: Mutex<Box<T>>,
    phantom: PhantomData<&'a E>,
}

impl<'a, C: ?Sized, T: ?Sized, E> BaseCollector<'a, C, T, E> {
    pub fn new(encoder: Box<C>, transport: Box<T>) -> Self {
        BaseCollector {
            max_message_size: 4096,
            encoder: Mutex::new(encoder),
            transport: Mutex::new(transport),
            phantom: PhantomData,
        }
    }
}

impl<'a, C, T, E> Collector for BaseCollector<'a, C, T, E>
    where C: Codec<Item = Vec<Span<'a>>, Error = E> + ?Sized + Send,
          T: Transport<Buffer = BytesMut, Output = (), Error = E> + ?Sized,
          E: From<::std::io::Error> + Send + Sync
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
                           HexViewBuilder::new(&buf[..])
                               .codepage(&CODEPAGE_HEX[..])
                               .finish()
                               .to_string()
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

    use super::{Encoder, Transport, Collector, BaseCollector, Span};
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

    impl Transport for MockTransport {
        type Buffer = BytesMut;
        type Output = ();
        type Error = Error;

        fn send(&mut self, buf: &BytesMut) -> Result<Self::Output, Self::Error> {
            self.sent += 1;
            self.buf.extend_from_slice(&buf[..]);

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
            encoder: Mutex::new(Box::new(MockEncoder::new())),
            transport: Mutex::new(Box::new(MockTransport::new())),
            phantom: PhantomData,
        };

        collector.submit(vec![span]).unwrap();

        assert_eq!(collector.encoder.lock().unwrap().encoded, 1);
        assert_eq!(collector.transport.lock().unwrap().sent, 1);
        assert_eq!(collector.transport.lock().unwrap().buf, b"hello world");
    }
}