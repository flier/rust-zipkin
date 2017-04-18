use std::rc::Rc;
use std::cell::RefCell;
use std::marker::PhantomData;

use tokio_io::codec::Encoder;

use bytes::{BytesMut, BufMut};

use mime::Mime;

use thrift::protocol::{TListIdentifier, TType, TOutputProtocol, TBinaryOutputProtocol};
use thrift::transport::{TBufferTransport, TPassThruTransport};

use errors::Error;
use encode::{ToThrift, to_writer};

use zipkin_core::{BatchEncoder, MimeType};

pub struct ThriftCodec<T, E> {
    item: PhantomData<T>,
    error: PhantomData<E>,
}

impl<T, E> ThriftCodec<T, E> {
    pub fn new() -> Self {
        ThriftCodec {
            item: PhantomData,
            error: PhantomData,
        }
    }
}

impl<T, E> Encoder for ThriftCodec<T, E>
    where T: ToThrift,
          E: From<::std::io::Error> + From<::thrift::Error> + From<Error>
{
    type Item = T;
    type Error = E;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut buf = dst.writer();

        to_writer(&mut buf, &item)?;

        Ok(())
    }
}

impl<T, E> BatchEncoder for ThriftCodec<T, E>
    where T: ToThrift,
          E: From<::std::io::Error> + From<::thrift::Error> + From<Error>
{
    fn batch_begin(&mut self, count: usize, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let buf = Rc::new(RefCell::new(Box::new(TBufferTransport::with_capacity(4, 4))));
        let mut proto =
            TBinaryOutputProtocol::new(Rc::new(RefCell::new(Box::new(TPassThruTransport {
                                                                         inner: buf.clone(),
                                                                     }))),
                                       true);
        proto
            .write_list_begin(&TListIdentifier::new(TType::Struct, count as i32))?;
        dst.put_slice(buf.borrow().read_buffer());
        Ok(())
    }

    fn batch_encode(&mut self, item: &[Self::Item], dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut buf = dst.writer();

        to_writer(&mut buf, &item)?;

        Ok(())
    }

    fn batch_end(&mut self, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let buf = Rc::new(RefCell::new(Box::new(TBufferTransport::with_capacity(4, 4))));
        let mut proto =
            TBinaryOutputProtocol::new(Rc::new(RefCell::new(Box::new(TPassThruTransport {
                                                                         inner: buf.clone(),
                                                                     }))),
                                       true);
        proto.write_list_end()?;
        dst.put_slice(buf.borrow().read_buffer());
        Ok(())
    }
}

impl<T, E> MimeType for ThriftCodec<T, E> {
    fn mime_type(&self) -> Mime {
        mime!(Application / ("x-thrift"))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use bytes::BytesMut;

    use zipkin_core::*;

    use super::*;

    #[test]
    fn encoder() {
        let mut span = Span::new("test")
            .with_trace_id(TraceId {
                               lo: 123,
                               hi: Some(456),
                           })
            .with_id(123)
            .with_parent_id(456)
            .with_debug(true);
        let endpoint =
            Some(Arc::new(Endpoint {
                              name: Some("test"),
                              addr: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                                                         8080)),
                          }));

        span.annotate(CLIENT_SEND, endpoint.clone());
        span.binary_annotate(HTTP_METHOD, "GET", endpoint.clone());

        let mut codec = ThriftCodec::<_, Error>::new();
        let mut buf = BytesMut::with_capacity(1024);

        codec.encode(span, &mut buf).unwrap();
    }
}