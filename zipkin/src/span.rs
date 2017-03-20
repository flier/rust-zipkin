use std::net::SocketAddr;

use chrono;
use chrono::prelude::*;

/// Unique identifier for a trace, set on all spans within it.
#[derive(Clone, Debug)]
pub struct TraceId {
    pub lo: u64,
    pub hi: Option<u64>,
}

/// Unique 8-byte identifier of this span within a trace.
pub type SpanId = u64;

/// Epoch microseconds
pub type Timestamp = DateTime<UTC>;

pub type Duration = chrono::Duration;

/// Indicates the network context of a service recording an annotation with two exceptions.
#[derive(Clone, Debug)]
pub struct Endpoint<'a> {
    /// Classifier of a source or destination in lowercase, such as "zipkin-server".
    pub service_name: Option<&'a str>,
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
    pub endpoint: Option<&'a Endpoint<'a>>,
}

#[derive(Clone, Debug)]
pub enum Value<'a> {
    Bool(bool),
    Bytes(&'a [u8]),
    I16(i16),
    I32(i32),
    I64(i64),
    Double(f64),
    String(&'a str),
}

#[derive(Clone, Debug)]
pub struct BinaryAnnotation<'a> {
    /// Name used to lookup spans
    pub key: &'a str,
    /// Value of annotation
    pub value: Value<'a>,
    /// The host that recorded, primarily for query by service name.
    pub endpoint: Option<&'a Endpoint<'a>>,
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
    pub timestamp: Option<Timestamp>,
    /// Measurement in microseconds of the critical path, if known.
    /// Durations of less than one microsecond must be rounded up to 1 microsecond.
    pub duration: Option<Duration>,
    /// Associates events that explain latency with a timestamp.
    pub annotations: Vec<Annotation<'a>>,
    /// Tags a span with context, usually to support query or aggregation.
    pub binary_annotations: Vec<BinaryAnnotation<'a>>,
    /// A request to store this span even if it overrides sampling policy.
    pub debug: Option<bool>,
}