use std::str;
use std::io::prelude::*;
use std::string::String;
use std::net::SocketAddr;

use serde_json;
use serde_json::{Map, Value, Result};

use base64;

use zipkin_core::{self as zipkin, TraceId, SpanId, Timestamp, ToMicrosecond, Duration, Endpoint,
                  Annotation, BinaryAnnotation, Span};

pub trait ToJson {
    fn to_json(&self) -> Value;
}

impl ToJson for TraceId {
    fn to_json(&self) -> Value {
        let id = match self.hi {
            Some(hi) => format!("{:016x}{:016x}", hi, self.lo),
            None => format!("{:016x}", self.lo),
        };

        id.into()
    }
}

impl ToJson for SpanId {
    fn to_json(&self) -> Value {
        format!("{:016x}", self).into()
    }
}

impl ToJson for Timestamp {
    fn to_json(&self) -> Value {
        self.to_microseconds().into()
    }
}

impl ToJson for Duration {
    fn to_json(&self) -> Value {
        self.num_milliseconds().into()
    }
}

impl<'a> ToJson for Endpoint<'a> {
    fn to_json(&self) -> Value {
        let mut attrs = Map::new();

        if let Some(name) = self.name {
            attrs.insert("serviceName".into(), name.into());
        }

        match self.addr {
            Some(SocketAddr::V4(addr)) => {
                attrs.insert("ipv4".into(), addr.ip().to_string().into());

                if addr.port() > 0 {
                    attrs.insert("port".into(), addr.port().into());
                }
            }
            Some(SocketAddr::V6(addr)) => {
                attrs.insert("ipv6".into(), addr.ip().to_string().into());

                if addr.port() > 0 {
                    attrs.insert("port".into(), addr.port().into());
                }
            }
            None => {}
        }

        attrs.into()
    }
}

impl<'a> ToJson for Annotation<'a> {
    fn to_json(&self) -> Value {
        let mut attrs = Map::new();

        attrs.insert("timestamp".into(), self.timestamp.to_json());
        attrs.insert("value".into(), self.value.into());
        if let Some(ref endpoint) = self.endpoint {
            attrs.insert("endpoint".into(), endpoint.to_json());
        }

        attrs.into()
    }
}

impl<'a> ToJson for BinaryAnnotation<'a> {
    fn to_json(&self) -> Value {
        let mut attrs = Map::new();

        attrs.insert("key".into(), self.key.into());

        let (value, ty) = match self.value {
            zipkin::Value::Bool(v) => (v.into(), None),
            zipkin::Value::Bytes(v) => (base64::encode(v).into(), Some("BYTES")),
            zipkin::Value::I16(v) => (v.into(), Some("I16")),
            zipkin::Value::I32(v) => (v.into(), Some("I32")),
            zipkin::Value::I64(v) => (v.into(), Some("I64")),
            zipkin::Value::Double(v) => (v.into(), Some("DOUBLE")),
            zipkin::Value::Str(v) => (v.into(), None),
            zipkin::Value::String(ref v) => (v.clone().into(), None),
        };

        attrs.insert("value".into(), value);

        if let Some(ty) = ty {
            attrs.insert("type".into(), ty.into());
        }
        if let Some(ref endpoint) = self.endpoint {
            attrs.insert("endpoint".into(), endpoint.to_json());
        }

        attrs.into()
    }
}

impl<'a> ToJson for Span<'a> {
    fn to_json(&self) -> Value {
        let mut attrs = Map::new();

        attrs.insert("traceId".into(), self.trace_id.to_json());
        attrs.insert("id".into(), self.id.to_json());
        attrs.insert("name".into(), self.name.into());
        if let Some(id) = self.parent_id {
            attrs.insert("parentId".into(), id.to_json());
        }
        attrs.insert("timestamp".into(), self.timestamp.to_json());
        if let Some(d) = self.duration {
            attrs.insert("duration".into(), d.to_json());
        }
        if !self.annotations.is_empty() {
            attrs.insert("annotations".into(),
                         self.annotations
                             .iter()
                             .map(|annotation| annotation.to_json())
                             .collect::<Vec<Value>>()
                             .into());
        }
        if !self.binary_annotations.is_empty() {
            attrs.insert("binaryAnnotations".into(),
                         self.binary_annotations
                             .iter()
                             .map(|annotation| annotation.to_json())
                             .collect::<Vec<Value>>()
                             .into());
        }
        if let Some(debug) = self.debug {
            attrs.insert("debug".into(), debug.into());
        }

        attrs.into()
    }
}

impl<'a, T: ToJson> ToJson for &'a [T] {
    fn to_json(&self) -> Value {
        self.iter()
            .map(|item| item.to_json())
            .collect::<Vec<Value>>()
            .into()
    }
}

pub fn to_json<T: ToJson>(value: &T) -> Value {
    value.to_json()
}

pub fn to_string<T: ToJson>(value: &T) -> Result<String> {
    serde_json::ser::to_string(&value.to_json())
}

pub fn to_string_pretty<T: ToJson>(value: &T) -> Result<String> {
    serde_json::ser::to_string_pretty(&value.to_json())
}

pub fn to_vec<T: ToJson>(value: &T) -> Result<Vec<u8>> {
    serde_json::ser::to_vec(&value.to_json())
}

pub fn to_vec_pretty<T: ToJson>(value: &T) -> Result<Vec<u8>> {
    serde_json::ser::to_vec_pretty(&value.to_json())
}

pub fn to_writer<W: ?Sized + Write, T: ToJson>(writer: &mut W, value: &T) -> Result<()> {
    serde_json::ser::to_writer(writer, &value.to_json())
}

pub fn to_writer_pretty<W: ?Sized + Write, T: ToJson>(writer: &mut W, value: &T) -> Result<()> {
    serde_json::ser::to_writer_pretty(writer, &value.to_json())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use diff;

    use zipkin_core::*;

    use super::*;

    #[test]
    fn to_json() {
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


        span.annotations[0].timestamp = timestamp(0, 0);
        span.annotations[1].timestamp = timestamp(0, 0);
        span.timestamp = timestamp(0, 0);

        let json = to_string_pretty(&span).unwrap();
        let diffs: Vec<String> = diff::lines(&json,
                                             unsafe { str::from_utf8_unchecked(PRETTY_JSON) })
                .iter()
                .flat_map(|ref line| match **line {
                              diff::Result::Both(..) => None,
                              diff::Result::Left(s) => Some(format!("-{}", s)),
                              diff::Result::Right(s) => Some(format!("+{}", s)),
                          })
                .collect();

        assert_eq!(diffs, Vec::<String>::new());
    }

    const PRETTY_JSON: &'static [u8] = br#"{
  "annotations": [
    {
      "endpoint": {
        "ipv4": "127.0.0.1",
        "port": 8080,
        "serviceName": "test"
      },
      "timestamp": 0,
      "value": "cs"
    },
    {
      "timestamp": 0,
      "value": "cr"
    }
  ],
  "binaryAnnotations": [
    {
      "endpoint": {
        "ipv4": "127.0.0.1",
        "port": 8080,
        "serviceName": "test"
      },
      "key": "http.method",
      "value": "GET"
    },
    {
      "key": "debug",
      "value": true
    },
    {
      "key": "http.status_code",
      "type": "I16",
      "value": 123
    },
    {
      "key": "http.request.size",
      "type": "I32",
      "value": -456
    },
    {
      "key": "http.response.size",
      "type": "I64",
      "value": -789
    },
    {
      "key": "time",
      "type": "DOUBLE",
      "value": 123.456
    },
    {
      "key": "raw",
      "type": "BYTES",
      "value": "c29tZQByYXcAZGF0YQ=="
    }
  ],
  "debug": true,
  "id": "000000000000007b",
  "name": "test",
  "parentId": "00000000000001c8",
  "timestamp": 0,
  "traceId": "00000000000001c8000000000000007b"
}"#;
}