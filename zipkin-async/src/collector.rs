use std::sync::{Arc, Mutex};

use futures::future;
use futures::{Future, BoxFuture};
use futures_cpupool::CpuPool;

use bytes::BytesMut;

use tokio_io::codec::Encoder;

use zipkin::{Span, Transport};

use errors::Error;

#[derive(Clone)]
pub struct AsyncCollector<E, T> {
    pub max_message_size: usize,
    pub encoder: E,
    pub transport: Arc<Mutex<T>>,
    pub thread_pool: CpuPool,
}

impl<'a, E, T> AsyncCollector<E, T>
    where E: Encoder<Item = Span<'a>, Error = Error>,
          T: Transport<BytesMut, Error = Error>
{
    pub fn submit(&mut self, span: Span<'a>) -> BoxFuture<(), Error> {
        let mut buf = BytesMut::with_capacity(self.max_message_size);

        if let Err(err) = self.encoder.encode(span, &mut buf) {
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
    fn submit() {
        let span = Span::new("test");

        let mut collector = AsyncCollector {
            max_message_size: 1024,
            encoder: MockEncoder::new(),
            transport: Arc::new(Mutex::new(MockTransport::new())),
            thread_pool: CpuPool::new(1),
        };

        collector.submit(span).wait().unwrap();

        assert_eq!(collector.encoder.encoded, 1);
        assert_eq!(collector.transport.lock().unwrap().sent, 1);
        assert_eq!(collector.transport.lock().unwrap().buf, b"hello world");
    }
}