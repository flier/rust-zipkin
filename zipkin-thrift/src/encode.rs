use std::ptr;
use std::mem;
use std::rc::Rc;
use std::ops::Deref;
use std::cell::RefCell;
use std::io::prelude::*;
use std::net::SocketAddr;

use byteorder::{BigEndian, WriteBytesExt};

use thrift;
use thrift::protocol::{TListIdentifier, TType, TOutputProtocol, TBinaryOutputProtocol};
use thrift::transport::{TBufferTransport, TPassThruTransport};

use zipkin_core as zipkin;
use zipkin_core::ToMicrosecond;

use core;
use errors::Result;

trait ToI64 {
    fn to_i64(&self) -> i64;
}

impl ToI64 for zipkin::Timestamp {
    fn to_i64(&self) -> i64 {
        self.to_microseconds()
    }
}

impl ToI64 for zipkin::Duration {
    fn to_i64(&self) -> i64 {
        self.num_milliseconds()
    }
}

pub trait ToThrift {
    type Output;

    fn to_thrift(&self) -> Self::Output;

    fn write_to(&self, proto: &mut TOutputProtocol) -> thrift::Result<()>;
}

impl<'a> ToThrift for zipkin::Endpoint<'a> {
    type Output = core::Endpoint;

    fn to_thrift(&self) -> Self::Output {
        core::Endpoint {
            service_name: self.name.map(|name| name.into()),
            ipv4: if let Some(SocketAddr::V4(addr)) = self.addr {
                let ip = &addr.ip().octets()[..];

                Some(unsafe { ptr::read(ip.as_ptr() as *const i32) })
            } else {
                None
            },
            ipv6: if let Some(SocketAddr::V6(addr)) = self.addr {
                let ip = &addr.ip().octets()[..];

                Some(ip.into())
            } else {
                None
            },
            port: self.addr.map(|addr| addr.port() as i16),
        }
    }

    fn write_to(&self, proto: &mut TOutputProtocol) -> thrift::Result<()> {
        self.to_thrift().write_to_out_protocol(proto)
    }
}

impl<'a> ToThrift for zipkin::Annotation<'a> {
    type Output = core::Annotation;

    fn to_thrift(&self) -> Self::Output {
        core::Annotation {
            timestamp: Some(self.timestamp.to_i64()),
            value: Some(self.value.into()),
            host: self.endpoint.to_thrift(),
        }
    }

    fn write_to(&self, proto: &mut TOutputProtocol) -> thrift::Result<()> {
        self.to_thrift().write_to_out_protocol(proto)
    }
}

impl<'a> ToThrift for zipkin::BinaryAnnotation<'a> {
    type Output = core::BinaryAnnotation;

    fn to_thrift(&self) -> Self::Output {
        let mut buf = vec![];
        let (value, ty) = match self.value {
            zipkin::Value::Bool(v) => (vec![if v { 1 } else { 0 }], core::AnnotationType::BOOL),
            zipkin::Value::Bytes(v) => (v.into(), core::AnnotationType::BYTES),
            zipkin::Value::I16(v) => {
                buf.write_i16::<BigEndian>(v).unwrap();

                (buf, core::AnnotationType::I16)
            }
            zipkin::Value::I32(v) => {
                buf.write_i32::<BigEndian>(v).unwrap();

                (buf, core::AnnotationType::I32)
            }
            zipkin::Value::I64(v) => {
                buf.write_i64::<BigEndian>(v).unwrap();

                (buf, core::AnnotationType::I64)
            }
            zipkin::Value::Double(v) => {
                let v: [u8; 8] = unsafe { mem::transmute(v) };

                buf.write(&v).unwrap();

                (buf, core::AnnotationType::DOUBLE)
            }
            zipkin::Value::Str(v) => (v.as_bytes().into(), core::AnnotationType::STRING),
            zipkin::Value::String(ref v) => (v.as_bytes().into(), core::AnnotationType::STRING),
        };

        core::BinaryAnnotation {
            key: Some(self.key.into()),
            value: Some(value),
            annotation_type: Some(ty),
            host: self.endpoint.to_thrift(),
        }
    }

    fn write_to(&self, proto: &mut TOutputProtocol) -> thrift::Result<()> {
        self.to_thrift().write_to_out_protocol(proto)
    }
}

impl<'a> ToThrift for zipkin::Span<'a> {
    type Output = core::Span;

    fn to_thrift(&self) -> Self::Output {
        core::Span {
            trace_id: Some(self.trace_id.lo as i64),
            trace_id_high: self.trace_id.hi.map(|id| id as i64),
            name: Some(self.name.into()),
            id: Some(self.id as i64),
            parent_id: self.parent_id.map(|id| id as i64),
            annotations: self.annotations.to_thrift(),
            binary_annotations: self.binary_annotations.to_thrift(),
            debug: self.debug,
            timestamp: Some(self.timestamp.to_i64()),
            duration: self.duration.map(|d| d.to_i64()),
        }
    }

    fn write_to(&self, proto: &mut TOutputProtocol) -> thrift::Result<()> {
        self.to_thrift().write_to_out_protocol(proto)
    }
}

impl<'a, T: ToThrift, D: Deref<Target = T>> ToThrift for Option<D> {
    type Output = Option<T::Output>;

    fn to_thrift(&self) -> Self::Output {
        self.as_ref().map(|item| item.to_thrift())
    }

    fn write_to(&self, proto: &mut TOutputProtocol) -> thrift::Result<()> {
        if let Some(ref item) = *self {
            item.write_to(proto)
        } else {
            Ok(())
        }
    }
}

impl<'a, T: ToThrift> ToThrift for [T] {
    type Output = Option<Vec<T::Output>>;

    fn to_thrift(&self) -> Self::Output {
        if self.is_empty() {
            None
        } else {
            Some(self.iter()
                     .map(|item| item.to_thrift())
                     .collect::<Vec<T::Output>>())
        }
    }

    fn write_to(&self, proto: &mut TOutputProtocol) -> thrift::Result<()> {
        proto
            .write_list_begin(&TListIdentifier::new(TType::Struct, self.len() as i32))?;

        for item in self {
            item.write_to(proto)?;
        }

        proto.write_list_end()
    }
}

pub fn to_thrift<T: ToThrift>(value: &T) -> T::Output {
    value.to_thrift()
}

pub fn to_vec<T: ToThrift>(value: &T) -> Result<Vec<u8>> {
    let buf = Rc::new(RefCell::new(Box::new(TBufferTransport::with_capacity(0, 4096))));
    let mut proto =
        TBinaryOutputProtocol::new(Rc::new(RefCell::new(Box::new(TPassThruTransport {
                                                                     inner: buf.clone(),
                                                                 }))),
                                   true);

    value.write_to(&mut proto)?;

    let bytes = buf.borrow_mut().write_buffer_to_vec();

    Ok(bytes)
}

pub fn to_writer<W: ?Sized + Write, T: ToThrift>(writer: &mut W, value: &T) -> Result<usize> {
    let buf = Rc::new(RefCell::new(Box::new(TBufferTransport::with_capacity(0, 4096))));
    let mut proto =
        TBinaryOutputProtocol::new(Rc::new(RefCell::new(Box::new(TPassThruTransport {
                                                                     inner: buf.clone(),
                                                                 }))),
                                   true);

    value.write_to(&mut proto)?;

    let wrote = writer.write(buf.borrow().write_buffer_as_ref())?;

    Ok(wrote)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use zipkin_core::*;

    use super::*;
    use super::super::core;

    #[test]
    fn to_thrift() {
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
        span.annotate(CLIENT_RECV, None);
        span.binary_annotate(HTTP_METHOD, "GET", endpoint.clone());
        span.binary_annotate("debug", true, None);
        span.binary_annotate(HTTP_STATUS_CODE, 123i16, None);
        span.binary_annotate(HTTP_REQUEST_SIZE, -456i32, None);
        span.binary_annotate(HTTP_RESPONSE_SIZE, -789i64, None);
        span.binary_annotate("time", 123.456, None);
        span.binary_annotate("raw", &b"some\0raw\0data"[..], None);

        span.annotations[0].timestamp = timestamp(123, 456);
        span.annotations[1].timestamp = timestamp(123, 456);
        span.timestamp = timestamp(123, 456);

        let msg = span.to_thrift();

        assert_eq!(msg.trace_id.unwrap(), 123);
        assert_eq!(msg.trace_id_high.unwrap(), 456);
        assert_eq!(msg.name.unwrap(), "test");
        assert_eq!(msg.id.unwrap(), 123);
        assert_eq!(msg.parent_id.unwrap(), 456);
        assert_eq!(msg.debug.unwrap(), true);
        assert_eq!(msg.timestamp.unwrap(), 123000000);
        assert!(msg.duration.is_none());

        let annotations = msg.annotations.unwrap();

        assert_eq!(annotations.len(), 2);
        assert_eq!(annotations[0].value.as_ref().unwrap(), CLIENT_SEND);
        assert_eq!(annotations[0].host.as_ref().unwrap().port.unwrap(), 8080);
        assert_eq!(annotations[1].value.as_ref().unwrap(), CLIENT_RECV);

        let annotations = msg.binary_annotations.unwrap();

        assert_eq!(annotations.len(), 7);
        assert_eq!(annotations[0].key.as_ref().unwrap(), HTTP_METHOD);
        assert_eq!(annotations[0].annotation_type.unwrap(),
                   core::AnnotationType::STRING);
        assert_eq!(annotations[0].host.as_ref().unwrap().port.unwrap(), 8080);

        assert_eq!(annotations[1].key.as_ref().unwrap(), "debug");
        assert_eq!(annotations[1].annotation_type.unwrap(),
                   core::AnnotationType::BOOL);

        assert_eq!(annotations[2].key.as_ref().unwrap(), HTTP_STATUS_CODE);
        assert_eq!(annotations[2].annotation_type.unwrap(),
                   core::AnnotationType::I16);

        assert_eq!(annotations[3].key.as_ref().unwrap(), HTTP_REQUEST_SIZE);
        assert_eq!(annotations[3].annotation_type.unwrap(),
                   core::AnnotationType::I32);

        assert_eq!(annotations[4].key.as_ref().unwrap(), HTTP_RESPONSE_SIZE);
        assert_eq!(annotations[4].annotation_type.unwrap(),
                   core::AnnotationType::I64);

        assert_eq!(annotations[5].key.as_ref().unwrap(), "time");
        assert_eq!(annotations[5].annotation_type.unwrap(),
                   core::AnnotationType::DOUBLE);

        assert_eq!(annotations[6].key.as_ref().unwrap(), "raw");
        assert_eq!(annotations[6].annotation_type.unwrap(),
                   core::AnnotationType::BYTES);

        let bytes = to_vec(&span).unwrap();

        assert_eq!(bytes.len(), 450);
    }
}