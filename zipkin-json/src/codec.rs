use std::marker::PhantomData;

use tokio_io::codec::Encoder;

use bytes::{BytesMut, BufMut};

use errors::Error;
use encode::{ToJson, to_writer, to_writer_pretty};

pub struct JsonCodec<T> {
    pub pretty_print: bool,
    phantom: PhantomData<T>,
}

impl<T> JsonCodec<T> {
    pub fn new() -> Self {
        JsonCodec {
            pretty_print: false,
            phantom: PhantomData,
        }
    }

    pub fn pretty() -> Self {
        JsonCodec {
            pretty_print: false,
            phantom: PhantomData,
        }
    }
}

impl<T> Encoder for JsonCodec<T>
    where T: ToJson
{
    type Item = T;
    type Error = Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut buf = dst.writer();

        if self.pretty_print {
            to_writer_pretty(&mut buf, &item)?;
        } else {
            to_writer(&mut buf, &item)?;
        }

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

        let mut codec = JsonCodec::new();
        let mut buf = BytesMut::with_capacity(1024);

        codec.encode(span, &mut buf).unwrap();
    }
}