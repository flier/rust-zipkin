use std::ptr;
use std::mem;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::prelude::*;
use std::net::SocketAddr;

use byteorder::{BigEndian, WriteBytesExt};

use thrift::protocol::TBinaryOutputProtocol;
use thrift::transport::{TBufferTransport, TPassThruTransport};

use zipkin;

use core;
use errors::Result;

trait Serialize {
    type Output;

    fn serialize(&self) -> Self::Output;
}

impl Serialize for zipkin::Timestamp {
    type Output = i64;

    fn serialize(&self) -> Self::Output {
        self.timestamp() * 1000_000 + self.timestamp_subsec_micros() as i64
    }
}

impl Serialize for zipkin::Duration {
    type Output = i64;

    fn serialize(&self) -> Self::Output {
        self.num_microseconds().unwrap_or(i64::max_value()).into()
    }
}

impl<'a> Serialize for zipkin::Endpoint<'a> {
    type Output = core::Endpoint;

    fn serialize(&self) -> Self::Output {
        core::Endpoint {
            service_name: self.service_name.map(|name| name.into()),
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
}

impl<'a> Serialize for zipkin::Annotation<'a> {
    type Output = core::Annotation;

    fn serialize(&self) -> Self::Output {
        core::Annotation {
            timestamp: Some(self.timestamp.serialize()),
            value: Some(self.value.into()),
            host: self.endpoint.map(|endpoint| endpoint.serialize()),
        }
    }
}

impl<'a> Serialize for zipkin::BinaryAnnotation<'a> {
    type Output = core::BinaryAnnotation;

    fn serialize(&self) -> Self::Output {
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
            zipkin::Value::String(v) => (v.as_bytes().into(), core::AnnotationType::STRING),
        };

        core::BinaryAnnotation {
            key: Some(self.key.into()),
            value: Some(value),
            annotation_type: Some(ty),
            host: self.endpoint.map(|endpoint| endpoint.serialize()),
        }
    }
}

impl<'a> Serialize for zipkin::Span<'a> {
    type Output = core::Span;

    fn serialize(&self) -> Self::Output {
        core::Span {
            trace_id: Some(self.trace_id.lo as i64),
            trace_id_high: self.trace_id.hi.map(|id| id as i64),
            name: Some(self.name.into()),
            id: Some(self.id as i64),
            parent_id: self.parent_id.map(|id| id as i64),
            annotations: if self.annotations.is_empty() {
                None
            } else {
                self.annotations
                    .iter()
                    .map(|annotation| annotation.serialize())
                    .collect::<Vec<core::Annotation>>()
                    .into()
            },
            binary_annotations: if self.binary_annotations.is_empty() {
                None
            } else {
                self.binary_annotations
                    .iter()
                    .map(|annotation| annotation.serialize())
                    .collect::<Vec<core::BinaryAnnotation>>()
                    .into()
            },
            debug: self.debug,
            timestamp: self.timestamp.map(|ts| ts.serialize()),
            duration: self.duration.map(|d| d.serialize()),
        }
    }
}

pub fn to_thrift(span: &zipkin::Span) -> core::Span {
    span.serialize()
}

pub fn to_vec(span: &zipkin::Span) -> Result<Vec<u8>> {
    let buf = Rc::new(RefCell::new(Box::new(TBufferTransport::with_capacity(0, 4096))));
    let mut proto =
        TBinaryOutputProtocol::new(Rc::new(RefCell::new(Box::new(TPassThruTransport {
                                       inner: buf.clone(),
                                   }))),
                                   true);

    to_thrift(span).write_to_out_protocol(&mut proto)?;

    let bytes = buf.borrow_mut().write_buffer_to_vec();

    Ok(bytes)
}

pub fn to_writer<W: ?Sized + Write>(writer: &mut W, span: &zipkin::Span) -> Result<usize> {
    let buf = Rc::new(RefCell::new(Box::new(TBufferTransport::with_capacity(0, 4096))));
    let mut proto =
        TBinaryOutputProtocol::new(Rc::new(RefCell::new(Box::new(TPassThruTransport {
                                       inner: buf.clone(),
                                   }))),
                                   true);

    to_thrift(span).write_to_out_protocol(&mut proto)?;

    let wrote = writer.write(buf.borrow().write_buffer_as_ref())?;

    Ok(wrote)
}