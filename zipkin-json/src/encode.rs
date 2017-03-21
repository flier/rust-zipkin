use std::str;
use std::string::String;
use std::io::prelude::*;
use std::net::SocketAddr;

use serde_json;
use serde_json::{Map, Value, Result};

use base64;

use zipkin;

trait ToJson {
    fn to_json(&self) -> Value;
}

impl ToJson for zipkin::TraceId {
    fn to_json(&self) -> Value {
        let id = match self.hi {
            Some(hi) => format!("{:016x}{:016x}", hi, self.lo),
            None => format!("{:016x}", self.lo),
        };

        id.into()
    }
}

impl ToJson for zipkin::SpanId {
    fn to_json(&self) -> Value {
        format!("{:016x}", self).into()
    }
}

impl ToJson for zipkin::Timestamp {
    fn to_json(&self) -> Value {
        let ts = self.timestamp() * 1000_000 + self.timestamp_subsec_micros() as i64;

        ts.into()
    }
}

impl ToJson for zipkin::Duration {
    fn to_json(&self) -> Value {
        self.num_microseconds().unwrap_or(i64::max_value()).into()
    }
}

impl<'a> ToJson for zipkin::Endpoint<'a> {
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

impl<'a> ToJson for zipkin::Annotation<'a> {
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

impl<'a> ToJson for zipkin::BinaryAnnotation<'a> {
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
            zipkin::Value::String(v) => (v.into(), None),
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

impl<'a> ToJson for zipkin::Span<'a> {
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

pub fn to_json(span: &zipkin::Span) -> Value {
    span.to_json()
}

pub fn to_string(span: &zipkin::Span) -> Result<String> {
    serde_json::ser::to_string(&span.to_json())
}

pub fn to_string_pretty(span: &zipkin::Span) -> Result<String> {
    serde_json::ser::to_string_pretty(&span.to_json())
}

pub fn to_vec(span: &zipkin::Span) -> Result<Vec<u8>> {
    serde_json::ser::to_vec(&span.to_json())
}

pub fn to_vec_pretty(span: &zipkin::Span) -> Result<Vec<u8>> {
    serde_json::ser::to_vec_pretty(&span.to_json())
}

pub fn to_writer<W: ?Sized + Write>(writer: &mut W, span: &zipkin::Span) -> Result<()> {
    serde_json::ser::to_writer(writer, &span.to_json())
}

pub fn to_writer_pretty<W: ?Sized + Write>(writer: &mut W, span: &zipkin::Span) -> Result<()> {
    serde_json::ser::to_writer_pretty(writer, &span.to_json())
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use chrono::prelude::*;
    use diff;

    use zipkin::*;

    use super::*;

    #[test]
    fn encode() {
        let mut span = Span::new("test")
            .with_trace_id(TraceId {
                lo: 123,
                hi: Some(456),
            })
            .with_id(123)
            .with_parent_id(456)
            .with_debug(true);
        let endpoint = Some(Rc::new(Endpoint {
            name: Some("test"),
            addr: Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)),
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


        span.annotations[0].timestamp = UTC.timestamp(0, 0);
        span.annotations[1].timestamp = UTC.timestamp(0, 0);
        span.timestamp = UTC.timestamp(0, 0);

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