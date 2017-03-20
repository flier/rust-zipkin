use std::string::String;
use std::io::prelude::*;
use std::net::SocketAddr;

use serde_json;
use serde_json::{Map, Value, Result};

use zipkin;

trait Serialize {
    fn serialize(&self) -> Value;
}

impl Serialize for zipkin::TraceId {
    fn serialize(&self) -> Value {
        let id = match self.hi {
            Some(hi) => format!("{:016x}{:016x}", hi, self.lo),
            None => format!("{:016x}", self.lo),
        };

        id.into()
    }
}

impl Serialize for zipkin::SpanId {
    fn serialize(&self) -> Value {
        format!("{:016x}", self).into()
    }
}

impl Serialize for zipkin::Timestamp {
    fn serialize(&self) -> Value {
        let ts = self.timestamp() * 1000_000 + self.timestamp_subsec_micros() as i64;

        ts.into()
    }
}

impl Serialize for zipkin::Duration {
    fn serialize(&self) -> Value {
        self.num_microseconds().unwrap_or(i64::max_value()).into()
    }
}

impl<'a> Serialize for zipkin::Endpoint<'a> {
    fn serialize(&self) -> Value {
        let mut attrs = Map::new();

        if let Some(name) = self.service_name {
            attrs["serviceName"] = name.into();
        }

        match self.addr {
            Some(SocketAddr::V4(addr)) => {
                attrs["ipv4"] = unsafe {
                        let ip = &addr.ip().octets()[..];

                        String::from_utf8_unchecked(ip.into())
                    }
                    .into();

                if addr.port() > 0 {
                    attrs["port"] = addr.port().into();
                }
            }
            Some(SocketAddr::V6(addr)) => {
                let ip = &addr.ip().octets()[..];

                attrs["ipv6"] = unsafe { String::from_utf8_unchecked(ip.into()) }.into();

                if addr.port() > 0 {
                    attrs["port"] = addr.port().into();
                }
            }
            None => {}
        }

        attrs.into()
    }
}

impl<'a> Serialize for zipkin::Annotation<'a> {
    fn serialize(&self) -> Value {
        let mut attrs = Map::new();

        attrs["timestamp"] = self.timestamp.serialize();
        attrs["value"] = self.value.into();
        if let Some(endpoint) = self.endpoint {
            attrs["endpoint"] = endpoint.serialize()
        }

        attrs.into()
    }
}

impl<'a> Serialize for zipkin::BinaryAnnotation<'a> {
    fn serialize(&self) -> Value {
        let mut attrs = Map::new();

        attrs["key"] = self.key.into();

        let (value, ty) = match self.value {
            zipkin::Value::Bool(v) => (v.into(), None),
            zipkin::Value::Bytes(v) => (v.into(), Some("BYTES")),
            zipkin::Value::I16(v) => (v.into(), Some("I16")),
            zipkin::Value::I32(v) => (v.into(), Some("I32")),
            zipkin::Value::I64(v) => (v.into(), Some("I64")),
            zipkin::Value::Double(v) => (v.into(), Some("DOUBLE")),
            zipkin::Value::String(v) => (v.into(), None),
        };

        attrs["value"] = value;
        if let Some(ty) = ty {
            attrs["type"] = ty.into()
        }
        if let Some(endpoint) = self.endpoint {
            attrs["endpoint"] = endpoint.serialize()
        }

        attrs.into()
    }
}

impl<'a> Serialize for zipkin::Span<'a> {
    fn serialize(&self) -> Value {
        let mut attrs = Map::new();

        attrs["traceId"] = self.trace_id.serialize();
        attrs["id"] = self.id.serialize();
        attrs["name"] = self.name.into();
        if let Some(id) = self.parent_id {
            attrs["parentId"] = id.serialize();
        }
        attrs["timestamp"] = self.timestamp.serialize();
        if let Some(d) = self.duration {
            attrs["duration"] = d.serialize();
        }
        if !self.annotations.is_empty() {
            attrs["annotations"] = self.annotations
                .iter()
                .map(|annotation| annotation.serialize())
                .collect::<Vec<Value>>()
                .into();
        }
        if !self.binary_annotations.is_empty() {
            attrs["binaryAnnotations"] = self.binary_annotations
                .iter()
                .map(|annotation| annotation.serialize())
                .collect::<Vec<Value>>()
                .into();
        }
        if let Some(debug) = self.debug {
            attrs["debug"] = debug.into();
        }

        attrs.into()
    }
}

pub fn to_json(span: &zipkin::Span) -> Value {
    span.serialize()
}

pub fn to_string(span: &zipkin::Span) -> Result<String> {
    serde_json::ser::to_string(&to_json(span))
}

pub fn to_vec(span: &zipkin::Span) -> Result<Vec<u8>> {
    serde_json::ser::to_vec(&to_json(span))
}

pub fn to_writer<W: ?Sized + Write>(writer: &mut W, span: &zipkin::Span) -> Result<()> {
    serde_json::ser::to_writer(writer, &to_json(span))
}