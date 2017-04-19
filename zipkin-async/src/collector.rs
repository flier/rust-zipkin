use std::sync::{Arc, Mutex, MutexGuard};
use std::marker::PhantomData;

use futures::future;
use futures::{Future, BoxFuture};
use futures_cpupool::CpuPool;

use bytes::BytesMut;

use zipkin_core::{Codec, Span, Transport, Collector};

use errors::Error;

pub trait AsyncCollector: Send {
    type Item;
    type Output;
    type Error;
    type Future: Future<Item = Self::Output, Error = Self::Error>;

    fn async_submit(&self, item: Self::Item) -> Self::Future;
}

#[inline(always)]
fn lock<T, F, E>(m: &Mutex<T>, callback: F) -> Result<(), E>
    where F: FnOnce(MutexGuard<T>) -> Result<(), E>,
          E: From<Error>
{
    match m.lock() {
        Ok(locked) => callback(locked),
        Err(err) => Err(Error::from(err).into()),
    }
}

#[derive(Clone)]
pub struct BaseAsyncCollector<C, T, E> {
    pub max_message_size: usize,
    pub encoder: Arc<Mutex<C>>,
    pub transport: Arc<Mutex<T>>,
    pub thread_pool: CpuPool,
    phantom: PhantomData<E>,
}

impl<'a, C, T, E> BaseAsyncCollector<C, T, E>
    where C: Codec<Item = Vec<Span<'a>>, Error = E>,
          E: From<::std::io::Error> + From<Error> + Send + Sync
{
    pub fn encode(&self, spans: Vec<Span<'a>>, buf: &mut BytesMut) -> Result<(), E> {
        lock(&self.encoder, |mut encoder| encoder.encode(spans, buf))
    }
}

impl<'a, C, T, E> Collector for BaseAsyncCollector<C, T, E>
    where C: Codec<Item = Vec<Span<'a>>, Error = E>,
          T: Transport<Buffer = BytesMut, Output = (), Error = E>,
          E: From<::std::io::Error> + From<Error> + Send + Sync
{
    type Item = Vec<Span<'a>>;
    type Output = ();
    type Error = E;

    fn submit(&self, spans: Self::Item) -> Result<Self::Output, Self::Error> {
        let mut buf = BytesMut::with_capacity(self.max_message_size);

        self.encode(spans, &mut buf)?;

        lock(&self.transport, |mut transport| transport.send(&buf))?;

        Ok(())
    }
}

impl<'a, C, T, E> AsyncCollector for BaseAsyncCollector<C, T, E>
    where C: 'static + Codec<Item = Vec<Span<'a>>, Error = E>,
          T: 'static + Transport<Buffer = BytesMut, Output = (), Error = E>,
          E: 'static + From<::std::io::Error> + From<Error> + Sync + Send
{
    type Item = Vec<Span<'a>>;
    type Output = ();
    type Error = E;
    type Future = BoxFuture<Self::Output, Self::Error>;

    fn async_submit(&self, spans: Self::Item) -> Self::Future {
        let mut buf = BytesMut::with_capacity(self.max_message_size);

        if let Err(err) = self.encode(spans, &mut buf) {
            return future::err(err).boxed();
        }

        let transport = self.transport.clone();

        self.thread_pool
            .spawn_fn(move || lock(&transport, |mut transport| transport.send(&buf)))
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

    use zipkin_core::{Encoder, Span, Transport};

    use super::{AsyncCollector, BaseAsyncCollector};
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

    impl Transport for MockTransport {
        type Buffer = BytesMut;
        type Output = ();
        type Error = Error;

        fn send(&mut self, buf: &BytesMut) -> ::std::result::Result<Self::Output, Self::Error> {
            self.sent += 1;
            self.buf.extend_from_slice(&buf[..]);

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
    fn async_submit() {
        let span = Span::new("test");

        let collector = BaseAsyncCollector {
            max_message_size: 1024,
            encoder: Arc::new(Mutex::new(MockEncoder::new())),
            transport: Arc::new(Mutex::new(MockTransport::new())),
            thread_pool: CpuPool::new(1),
            phantom: PhantomData,
        };

        collector.async_submit(vec![span]).wait().unwrap();

        assert_eq!(collector.encoder.lock().unwrap().encoded, 1);
        assert_eq!(collector.transport.lock().unwrap().sent, 1);
        assert_eq!(collector.transport.lock().unwrap().buf, b"hello world");
    }
}