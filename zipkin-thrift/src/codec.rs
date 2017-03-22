use std::marker::PhantomData;

use tokio_io::codec::Encoder;

use bytes::{BytesMut, BufMut};

use errors::Error;
use encode::{ToThrift, to_writer};

pub struct ThriftCodec<T>
    where T: ToThrift
{
    phantom: PhantomData<T>,
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