use std::mem;
use std::sync::Arc;
use std::cell::RefCell;
use std::time::Duration;
use std::net::SocketAddr;

use chrono::prelude::*;

use rand::{self, Rng};

use xoroshiro128::{SeedableRng, Xoroshiro128Rng};

/// Generate next id
///
/// It base on the same workflow from `std::collections::RandomState`
///
/// > Historically this function did not cache keys from the OS and instead
/// > simply always called `rand::thread_rng().gen()` twice. In #31356 it
/// > was discovered, however, that because we re-seed the thread-local RNG
/// > from the OS periodically that this can cause excessive slowdown when
/// > many hash maps are created on a thread. To solve this performance
/// > trap we cache the first set of randomly generated keys per-thread.
///
/// > Later in #36481 it was discovered that exposing a deterministic
/// > iteration order allows a form of DOS attack. To counter that we
/// > increment one of the seeds on every `RandomState` creation, giving
/// > every corresponding `HashMap` a different iteration order.
///
pub fn next_id() -> u64 {
    thread_local! {
        static SEEDS: RefCell<Xoroshiro128Rng> =
            RefCell::new(Xoroshiro128Rng::from_seed(rand::thread_rng().gen::<[u64; 2]>()));
    }

    SEEDS.with(|seeds| seeds.borrow_mut().next_u64())
}

/// Unique identifier for a trace, set on all spans within it.
#[derive(Clone, Debug)]
pub struct TraceId {
    pub lo: u64,
    pub hi: Option<u64>,
}

impl TraceId {
    pub fn gen() -> TraceId {
        TraceId {
            lo: next_id(),
            hi: Some(next_id()),
        }
    }
}

/// Unique 8-byte identifier of this span within a trace.
pub type SpanId = u64;

/// Epoch microseconds
pub type Timestamp = DateTime<UTC>;

/// Indicates the network context of a service recording an annotation with two exceptions.
#[derive(Clone, Debug)]
pub struct Endpoint<'a> {
    /// Classifier of a source or destination in lowercase, such as "zipkin-server".
    pub name: Option<&'a str>,
    /// Endpoint address packed in the network endian
    pub addr: Option<SocketAddr>,
}

/// Associates an event that explains latency with a timestamp.
#[derive(Clone, Debug)]
pub struct Annotation<'a> {
    /// Microseconds from epoch.
    pub timestamp: Timestamp,
    /// Usually a short tag indicating an event
    pub value: &'a str,
    /// The host that recorded, primarily for query by service name.
    pub endpoint: Option<Arc<Endpoint<'a>>>,
}

impl<'a> Annotation<'a> {
    fn new(value: &'a str, endpoint: Option<Arc<Endpoint<'a>>>) -> Annotation<'a> {
        Annotation {
            value: value,
            timestamp: UTC::now(),
            endpoint: endpoint,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Bool(bool),
    Bytes(&'a [u8]),
    I16(i16),
    I32(i32),
    I64(i64),
    Double(f64),
    Str(&'a str),
    String(String),
}

impl<'a> Value<'a> {
    pub fn as_bool(&self) -> Option<bool> {
        if let &Value::Bool(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        if let &Value::Bytes(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_i16(&self) -> Option<i16> {
        if let &Value::I16(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        if let &Value::I32(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        if let &Value::I64(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_u16(&self) -> Option<u16> {
        if let &Value::I16(v) = self {
            Some(v as u16)
        } else {
            None
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        if let &Value::I32(v) = self {
            Some(v as u32)
        } else {
            None
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        if let &Value::I64(v) = self {
            Some(v as u64)
        } else {
            None
        }
    }

    pub fn as_double(&self) -> Option<f64> {
        if let &Value::Double(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_str(&'a self) -> Option<&'a str> {
        match self {
            &Value::Str(v) => Some(v),
            &Value::String(ref v) => Some(v.as_str()),
            _ => None,
        }
    }
}

impl<'a> From<bool> for Value<'a> {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl<'a> From<i16> for Value<'a> {
    fn from(v: i16) -> Self {
        Value::I16(v)
    }
}

impl<'a> From<i32> for Value<'a> {
    fn from(v: i32) -> Self {
        Value::I32(v)
    }
}

impl<'a> From<i64> for Value<'a> {
    fn from(v: i64) -> Self {
        Value::I64(v)
    }
}

impl<'a> From<u16> for Value<'a> {
    fn from(v: u16) -> Self {
        Value::I16(v as i16)
    }
}

impl<'a> From<u32> for Value<'a> {
    fn from(v: u32) -> Self {
        Value::I32(v as i32)
    }
}

impl<'a> From<u64> for Value<'a> {
    fn from(v: u64) -> Self {
        Value::I64(v as i64)
    }
}

impl<'a> From<f64> for Value<'a> {
    fn from(v: f64) -> Self {
        Value::Double(v)
    }
}

impl<'a> From<f32> for Value<'a> {
    fn from(v: f32) -> Self {
        Value::Double(v as f64)
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(v: &'a str) -> Self {
        Value::Str(v)
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl<'a> From<&'a [u8]> for Value<'a> {
    fn from(v: &'a [u8]) -> Self {
        Value::Bytes(v)
    }
}

pub trait BinaryAnnotationValue<'a> {
    fn to_value(self) -> Value<'a>;
}

impl<'a, T> BinaryAnnotationValue<'a> for T
    where T: Into<Value<'a>>
{
    fn to_value(self) -> Value<'a> {
        self.into()
    }
}

#[derive(Clone, Debug)]
pub struct BinaryAnnotation<'a> {
    /// Name used to lookup spans
    pub key: &'a str,
    /// Value of annotation
    pub value: Value<'a>,
    /// The host that recorded, primarily for query by service name.
    pub endpoint: Option<Arc<Endpoint<'a>>>,
}

impl<'a> BinaryAnnotation<'a> {
    pub fn new<V>(key: &'a str,
                  value: V,
                  endpoint: Option<Arc<Endpoint<'a>>>)
                  -> BinaryAnnotation<'a>
        where V: Sized + BinaryAnnotationValue<'a>
    {
        BinaryAnnotation {
            key: key,
            value: value.to_value(),
            endpoint: endpoint,
        }
    }
}

/// A trace is a series of spans (often RPC calls) which form a latency tree.
#[derive(Clone, Debug)]
pub struct Span<'a> {
    /// Unique identifier for a trace, set on all spans within it.
    pub trace_id: TraceId,
    /// Span name in lowercase, rpc method for example.
    pub name: &'a str,
    /// Unique 8-byte identifier of this span within a trace.
    pub id: SpanId,
    /// The parent's id or None if this the root span in a trace.
    pub parent_id: Option<SpanId>,
    /// Epoch microseconds of the start of this span, possibly absent if this an incomplete span.
    pub timestamp: Timestamp,
    /// Measurement in microseconds of the critical path, if known.
    /// Durations of less than one microsecond must be rounded up to 1 microsecond.
    pub duration: Option<Duration>,
    /// Associates events that explain latency with a timestamp.
    pub annotations: Vec<Annotation<'a>>,
    /// Tags a span with context, usually to support query or aggregation.
    pub binary_annotations: Vec<BinaryAnnotation<'a>>,
    /// A request to store this span even if it overrides sampling policy.
    pub debug: Option<bool>,
    /// Span was sampled base on the sampling policy.
    pub sampled: Option<bool>,
}

impl<'a> Span<'a> {
    pub fn new(name: &'a str) -> Span<'a> {
        Span {
            trace_id: TraceId::gen(),
            name: name,
            id: next_id(),
            timestamp: UTC::now(),
            ..unsafe { mem::zeroed() }
        }
    }

    pub fn with_trace_id(self, trace_id: TraceId) -> Self {
        Span {
            trace_id: trace_id,
            ..self
        }
    }

    pub fn with_id(self, id: SpanId) -> Self {
        Span { id: id, ..self }
    }

    pub fn with_parent_id(self, parent_id: SpanId) -> Self {
        Span {
            parent_id: Some(parent_id),
            ..self
        }
    }

    pub fn with_debug(self, debug: bool) -> Self {
        Span {
            debug: Some(debug),
            ..self
        }
    }

    pub fn with_sampled(self, sampled: bool) -> Self {
        Span {
            sampled: Some(sampled),
            ..self
        }
    }
}

pub trait Annotatable<'a> {
    fn used(&self) -> bool;

    fn annotate(&mut self, value: &'a str, endpoint: Option<Arc<Endpoint<'a>>>);

    fn binary_annotate<V>(&mut self, key: &'a str, value: V, endpoint: Option<Arc<Endpoint<'a>>>)
        where V: Sized + BinaryAnnotationValue<'a>;
}

impl<'a> Annotatable<'a> for Span<'a> {
    fn used(&self) -> bool {
        self.debug == Some(true) || self.sampled != Some(false)
    }

    fn annotate(&mut self, value: &'a str, endpoint: Option<Arc<Endpoint<'a>>>) {
        self.annotations.push(Annotation::new(value, endpoint))
    }

    fn binary_annotate<V>(&mut self, key: &'a str, value: V, endpoint: Option<Arc<Endpoint<'a>>>)
        where V: Sized + BinaryAnnotationValue<'a>
    {
        self.binary_annotations
            .push(BinaryAnnotation::new(key, value, endpoint))
    }
}

impl<'a> Annotatable<'a> for Option<Span<'a>> {
    fn used(&self) -> bool {
        if let Some(Span { debug, sampled, .. }) = *self {
            debug == Some(true) || sampled != Some(false)
        } else {
            false
        }
    }

    fn annotate(&mut self, value: &'a str, endpoint: Option<Arc<Endpoint<'a>>>) {
        if let Some(&mut Span { ref mut annotations, .. }) = self.as_mut() {
            annotations.push(Annotation::new(value, endpoint))
        }
    }

    fn binary_annotate<V>(&mut self, key: &'a str, value: V, endpoint: Option<Arc<Endpoint<'a>>>)
        where V: Sized + BinaryAnnotationValue<'a>
    {
        if let Some(&mut Span { ref mut binary_annotations, .. }) = self.as_mut() {
            binary_annotations.push(BinaryAnnotation::new(key, value, endpoint))
        }
    }
}

include!("./macros.rs");

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use super::super::*;

    #[test]
    fn gen_id() {
        assert!(next_id() != 0);
        assert!(next_id() != next_id());

        let trace_id = TraceId::gen();

        assert!(trace_id.lo != 0);
        assert!(trace_id.hi.is_some());
        assert!(trace_id.hi.unwrap() != 0);
    }

    #[test]
    fn span() {
        let span = Span::new("test");

        assert!(span.trace_id.lo != 0);
        assert!(span.trace_id.hi.is_some());
        assert!(span.trace_id.hi.unwrap() != 0);

        assert_eq!(span.name, "test");

        assert!(span.id != 0);
        assert_eq!(span.parent_id, None);
        assert!(span.timestamp.timestamp() != 0);
        assert_eq!(span.duration, None);
        assert!(span.annotations.is_empty());
        assert!(span.binary_annotations.is_empty());
        assert_eq!(span.debug, None);

        assert_eq!(span.clone().with_id(123).id, 123);
        assert_eq!(span.clone().with_parent_id(456).parent_id, Some(456));
        assert_eq!(span.clone().with_debug(true).debug, Some(true));
    }

    #[test]
    fn annonation() {
        let mut span = Span::new("test");
        let endpoint = Some(Arc::new(Endpoint {
                                         name: Some("test"),
                                         addr: None,
                                     }));

        span.annotate(CLIENT_SEND, endpoint.clone());
        {
            let annonation = span.annotations.last().unwrap();

            assert_eq!(span.annotations.len(), 1);
            assert_eq!(annonation.value, CLIENT_SEND);
            assert!(annonation.timestamp.timestamp() != 0);
            assert!(annonation.endpoint.is_some());
            assert_eq!(annonation.endpoint.as_ref().unwrap().name, Some("test"));
        }
        span.annotate(CLIENT_RECV, None);
        {
            let annonation = span.annotations.last().unwrap();

            assert_eq!(span.annotations.len(), 2);
            assert_eq!(annonation.value, CLIENT_RECV);
            assert!(annonation.timestamp.timestamp() != 0);
            assert!(annonation.endpoint.is_none());
        }
        span.binary_annotate(HTTP_METHOD, "GET", endpoint.clone());
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 1);
            assert_eq!(annonation.key, HTTP_METHOD);
            assert_eq!(annonation.value, Value::Str("GET"));
        }
        span.binary_annotate("debug", true, None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 2);
            assert_eq!(annonation.key, "debug");
            assert_eq!(annonation.value, Value::Bool(true));
        }
        span.binary_annotate(HTTP_STATUS_CODE, 123i16, None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 3);
            assert_eq!(annonation.key, HTTP_STATUS_CODE);
            assert_eq!(annonation.value, Value::I16(123));
        }
        span.binary_annotate(HTTP_REQUEST_SIZE, -456i32, None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 4);
            assert_eq!(annonation.key, HTTP_REQUEST_SIZE);
            assert_eq!(annonation.value, Value::I32(-456));
        }
        span.binary_annotate(HTTP_RESPONSE_SIZE, -789i64, None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 5);
            assert_eq!(annonation.key, HTTP_RESPONSE_SIZE);
            assert_eq!(annonation.value, Value::I64(-789));

        }
        span.binary_annotate("time", 123.456, None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 6);
            assert_eq!(annonation.key, "time");
            assert_eq!(annonation.value, Value::Double(123.456));
        }
        span.binary_annotate(ERROR, "some error", None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 7);
            assert_eq!(annonation.key, ERROR);
            assert_eq!(annonation.value, Value::Str("some error"));
        }
        span.binary_annotate("raw", &b"some\0raw\0data"[..], None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 8);
            assert_eq!(annonation.key, "raw");
            assert_eq!(annonation.value, Value::Bytes(&b"some\0raw\0data"[..]));
        }
        span.binary_annotate(HTTP_STATUS_CODE, i16::max_value() as u16 + 1, None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 9);
            assert_eq!(annonation.key, HTTP_STATUS_CODE);
            assert_eq!(annonation.value, Value::I16(-32768));
            assert_eq!(annonation.value.as_u16(), Some(0x8000));
        }
        span.binary_annotate(HTTP_REQUEST_SIZE, i32::max_value() as u32 + 1, None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 10);
            assert_eq!(annonation.key, HTTP_REQUEST_SIZE);
            assert_eq!(annonation.value, Value::I32(-2147483648));
            assert_eq!(annonation.value.as_u32(), Some(0x80000000));
        }
        span.binary_annotate(HTTP_RESPONSE_SIZE, i64::max_value() as u64 + 1, None);
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 11);
            assert_eq!(annonation.key, HTTP_RESPONSE_SIZE);
            assert_eq!(annonation.value, Value::I64(-9223372036854775808));
            assert_eq!(annonation.value.as_u64(), Some(0x8000000000000000));
        }
    }

    #[test]
    fn macros() {
        let mut span = Span::new("test");
        let endpoint = Some(Arc::new(Endpoint {
                                         name: Some("test"),
                                         addr: None,
                                     }));

        annotate!(span, CLIENT_SEND);
        {
            let annonation = span.annotations.last().unwrap();

            assert_eq!(span.annotations.len(), 1);
            assert_eq!(annonation.value, CLIENT_SEND);
        }

        annotate!(span, CLIENT_RECV, endpoint => endpoint.clone());
        {
            let annonation = span.annotations.last().unwrap();

            assert_eq!(span.annotations.len(), 2);
            assert_eq!(annonation.value, CLIENT_RECV);
            assert_eq!(annonation.endpoint.as_ref().unwrap().name, Some("test"));
        }

        annotate!(span, HTTP_METHOD, "GET");
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 1);
            assert_eq!(annonation.key, HTTP_METHOD);
            assert_eq!(annonation.value, Value::Str("GET"));
        }

        annotate!(span, HTTP_STATUS_CODE, 123i16, endpoint => endpoint.clone());
        {
            let annonation = span.binary_annotations.last().unwrap();

            assert_eq!(span.binary_annotations.len(), 2);
            assert_eq!(annonation.key, HTTP_STATUS_CODE);
            assert_eq!(annonation.value, Value::I16(123));
            assert_eq!(annonation.endpoint.as_ref().unwrap().name, Some("test"));
        }

        span = span.with_sampled(false);

        annotate!(span, CLIENT_SEND);
        {
            assert_eq!(span.annotations.len(), 2);
        }

        span = span.with_debug(true);

        annotate!(span, HTTP_METHOD, "GET");
        {
            assert_eq!(span.binary_annotations.len(), 3);
        }

        let mut span = Some(span);

        annotate!(span, CLIENT_RECV_FRAGMENT);
        {
            assert_eq!(span.map(|span| {
                                    (span.annotations.len(), span.annotations.last().unwrap().value)
                                })
                           .unwrap(),
                       (3, CLIENT_RECV_FRAGMENT));
        }

        span = None;

        annotate!(span, CLIENT_RECV_FRAGMENT);
    }
}