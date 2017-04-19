use std::marker::PhantomData;

use bytes::{BytesMut, BufMut};

use mime::Mime;

use errors::Error;
use encode::{ToThrift, to_writer};

use zipkin_core::{Encoder, MimeType};

pub struct ThriftCodec<T, E>(PhantomData<(T, E)>);

unsafe impl<T, E> Send for ThriftCodec<T, E> {}

impl<T, E> ThriftCodec<T, E> {
    pub fn new() -> Self {
        ThriftCodec(PhantomData)
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