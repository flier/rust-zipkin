use std::mem;
use std::rc::Rc;
use std::cell::RefCell;
use std::net::SocketAddr;

use chrono;
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

pub type Duration = chrono::Duration;

/// Indicates the network context of a service recording an annotation with two exceptions.
#[derive(Clone, Debug)]
pub struct Endpoint<'a> {
    /// Classifier of a source or destination in lowercase, such as "zipkin-server".
    pub service_name: Option<&'a str>,
    /// Endpoint address packed in the network endian
    pub addr: Option<SocketAddr>,
}

pub trait AnnotationValue: AsRef<str> {}

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

impl<'a> Annotation<'a> {
    fn new(value: &'a str) -> Annotation<'a> {
        Annotation {
            value: value,
            timestamp: UTC::now(),
            endpoint: None,
        }
    }

    pub fn with_endpoint(&mut self, endpoint: &'a Endpoint) -> &mut Self {
        self.endpoint = Some(endpoint);
        self
    }
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

impl<'a> BinaryAnnotation<'a> {
    pub fn new<K, V>(key: &K, value: &V) -> BinaryAnnotation<'a>
        where K: AsRef<&'a str>,
              V: BinaryAnnotationValue
    {
        BinaryAnnotation {
            key: key.as_ref(),
            value: unsafe { mem::zeroed() },
            endpoint: None,
        }
    }

    pub fn with_endpoint(&mut self, endpoint: &'a Endpoint) -> &mut Self {
        self.endpoint = Some(endpoint);
        self
    }
}

pub trait BinaryAnnotationValue {}

impl BinaryAnnotationValue for bool {}
impl BinaryAnnotationValue for i16 {}
impl BinaryAnnotationValue for i32 {}
impl BinaryAnnotationValue for i64 {}
impl BinaryAnnotationValue for u16 {}
impl BinaryAnnotationValue for u32 {}
impl BinaryAnnotationValue for u64 {}
impl BinaryAnnotationValue for f32 {}
impl BinaryAnnotationValue for f64 {}
impl<'a> BinaryAnnotationValue for &'a str {}
impl<'a> BinaryAnnotationValue for &'a [u8] {}

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
    pub annotations: Vec<Rc<Annotation<'a>>>,
    /// Tags a span with context, usually to support query or aggregation.
    pub binary_annotations: Vec<Rc<BinaryAnnotation<'a>>>,
    /// A request to store this span even if it overrides sampling policy.
    pub debug: Option<bool>,
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

    pub fn with_trace_id(&mut self, trace_id: TraceId) -> &mut Self {
        self.trace_id = trace_id;
        self
    }

    pub fn with_span_id(&mut self, span_id: SpanId) -> &mut Self {
        self.id = span_id;
        self
    }

    pub fn with_parent_id(&mut self, parent_id: SpanId) -> &mut Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_debug(&mut self, debug: bool) -> &mut Self {
        self.debug = Some(debug);
        self
    }

    pub fn annonate<T>(&mut self, value: &'a T) -> Rc<Annotation<'a>>
        where T: AnnotationValue
    {
        let annotation = Rc::new(Annotation::new(value.as_ref()));

        self.annotations.push(annotation.clone());

        annotation
    }

    pub fn binary_annonate<K, T>(&mut self, key: &K, value: &'a T) -> Rc<BinaryAnnotation<'a>>
        where K: AsRef<&'a str>,
              T: BinaryAnnotationValue
    {
        let annotation = Rc::new(BinaryAnnotation::new(key, value));

        self.binary_annotations.push(annotation.clone());

        annotation
    }
}