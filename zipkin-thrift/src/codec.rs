use std::marker::PhantomData;

use tokio_io::codec::Encoder;

use bytes::{BytesMut, BufMut};

use errors::Error;
use encode::{ToThrift, to_writer};

pub struct ThriftCodec<T> {
    phantom: PhantomData<T>,
}

impl<T> ThriftCodec<T> {
    pub fn new() -> Self {
        ThriftCodec { phantom: PhantomData }
    }
}

impl<T> Encoder for ThriftCodec<T>
    where T: ToThrift
{
    type Item = T;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut buf = dst.writer();

        to_writer(&mut buf, &item)?;

        Ok(())
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
        let endpoint = Some(Arc::new(Endpoint {
            name: Some("test"),
            addr: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)),
        }));

        span.annotate(CLIENT_SEND, endpoint.clone());
        span.binary_annotate(HTTP_METHOD, "GET", endpoint.clone());

        let mut codec = ThriftCodec::new();
        let mut buf = BytesMut::with_capacity(1024);

        codec.encode(span, &mut buf).unwrap();
    }
}