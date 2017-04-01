use std::sync::{Arc, Mutex};

use futures::future;
use futures::{Future, BoxFuture};
use futures_cpupool::CpuPool;

use bytes::BytesMut;

use tokio_io::codec::Encoder;

use zipkin::{Span, Transport, Collector};

use errors::Error;

pub trait AsyncCollector {
    type Item;
    type Output: Send;
    type Error;
    type Result: Future<Item = Self::Output, Error = Self::Error>;

    fn async_submit(&mut self, item: Self::Item) -> Self::Result;
}

#[derive(Clone)]
pub struct BaseAsyncCollector<E, T> {
    pub max_message_size: usize,
    pub encoder: E,
    pub transport: Arc<Mutex<T>>,
    pub thread_pool: CpuPool,
}

impl<'a, E, T> Collector for BaseAsyncCollector<E, T>
    where E: Encoder<Item = Span<'a>, Error = Error>,
          T: Transport<BytesMut, Error = Error>
{
    type Item = Span<'a>;
    type Output = ();
    type Error = Error;

    fn submit(&mut self, span: Span<'a>) -> Result<Self::Output, Self::Error> {
        let mut buf = BytesMut::with_capacity(self.max_message_size);

        self.encoder.encode(span, &mut buf)?;

        self.transport.lock()?.send(&buf)?;

        Ok(())
    }
}

impl<'a, E, T> AsyncCollector for BaseAsyncCollector<E, T>
    where E: Encoder<Item = Span<'a>, Error = Error>,
          T: Transport<BytesMut, Error = Error>
{
    type Item = Span<'a>;
    type Output = ();
    type Error = Error;
    type Result = BoxFuture<Self::Output, Self::Error>;

    fn async_submit(&mut self, item: Self::Item) -> Self::Result {
        let mut buf = BytesMut::with_capacity(self.max_message_size);

        if let Err(err) = self.encoder.encode(item, &mut buf) {
            return future::err(err).boxed();
        }

        let transport = self.transport.clone();

        self.thread_pool
            .spawn_fn(move || {
                transport.lock()?.send(&buf)?;

                Ok(())
            })
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::marker::PhantomData;

    use bytes::{BytesMut, BufMut};

    use futures::Future;
    use futures_cpupool::CpuPool;

    use tokio_io::codec::Encoder;

    use zipkin::*;

    use super::super::*;
    use super::super::errors::Error;

    #[derive(Clone)]
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

    #[derive(Clone)]
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
    fn async_submit() {
        let span = Span::new("test");

        let mut collector = BaseAsyncCollector {
            max_message_size: 1024,
            encoder: MockEncoder::new(),
            transport: Arc::new(Mutex::new(MockTransport::new())),
            thread_pool: CpuPool::new(1),
        };

        collector.async_submit(span).wait().unwrap();

        assert_eq!(collector.encoder.encoded, 1);
        assert_eq!(collector.transport.lock().unwrap().sent, 1);
        assert_eq!(collector.transport.lock().unwrap().buf, b"hello world");
    }
}