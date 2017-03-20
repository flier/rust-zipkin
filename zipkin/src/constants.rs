/// The client sent ("cs") a request to a server.
///
/// There is only one send per span. For example, if there's a transport error,
/// each attempt can be logged as a {@link #WIRE_SEND} annotation.
pub const CLIENT_SEND: &'static str = "cs";

/// The client received ("cr") a response from a server.
///
/// There is only one receive per span. For example, if duplicate responses were received,
/// each can be logged as a {@link #WIRE_RECV} annotation.
pub const CLIENT_RECV: &'static str = "cr";

/// The server sent ("ss") a response to a client.
///
/// There is only one response per span. If there's a transport error,
/// each attempt can be logged as a {@link #WIRE_SEND} annotation.
pub const SERVER_SEND: &'static str = "ss";

/// The server received ("sr") a request from a client.
///
/// There is only one request per span.  For example, if duplicate responses were received,
/// each can be logged as a {@link #WIRE_RECV} annotation.
pub const SERVER_RECV: &'static str = "sr";

/// Optionally logs an attempt to send a message on the wire.
///
/// Multiple wire send events could indicate network retries.
/// A lag between client or server send and wire send might indicate queuing or processing delay.
pub const WIRE_SEND: &'static str = "ws";

/// Optionally logs an attempt to receive a message from the wire.
///
/// Multiple wire receive events could indicate network retries.
/// A lag between wire receive and client or server receive might indicate queuing or processing delay.
pub const WIRE_RECV: &'static str = "wr";

/// Optionally logs progress of a ({@linkplain #CLIENT_SEND}, {@linkplain #WIRE_SEND}).
///
/// For example, this could be one chunk in a chunked request.
pub const CLIENT_SEND_FRAGMENT: &'static str = "csf";

/// Optionally logs progress of a ({@linkplain #CLIENT_RECV}, {@linkplain #WIRE_RECV}).
///
/// For example, this could be one chunk in a chunked response.
pub const CLIENT_RECV_FRAGMENT: &'static str = "crf";

/// Optionally logs progress of a ({@linkplain #SERVER_SEND}, {@linkplain #WIRE_SEND}).
///
/// For example, this could be one chunk in a chunked response.
pub const SERVER_SEND_FRAGMENT: &'static str = "ssf";

/// Optionally logs progress of a ({@linkplain #SERVER_RECV}, {@linkplain #WIRE_RECV}).
///
/// For example, this could be one chunk in a chunked request.
pub const SERVER_RECV_FRAGMENT: &'static str = "srf";

/// The {@link BinaryAnnotation#value value} of "lc" is the component or namespace of a local span.
pub const LOCAL_COMPONENT: &'static str = "lc";

/// When an {@link Annotation#value}, this indicates when an error occurred.
///
/// When a {@link BinaryAnnotation#key}, the value is a human readable message associated with an error.
pub const ERROR: &'static str = "error";

/// When present, {@link BinaryAnnotation#endpoint} indicates a client address ("ca") in a span.
///
/// Most likely, there's only one. Multiple addresses are possible when a client changes its ip or port within a span.
pub const CLIENT_ADDR: &'static str = "ca";

/// When present, {@link BinaryAnnotation#endpoint} indicates a server address ("sa") in a span.
///
/// Most likely, there's only one. Multiple addresses are possible when a client is redirected,
/// or fails to a different server ip or port.
pub const SERVER_ADDR: &'static str = "sa";

/// Zipkin's core annotations indicate when a client or server operation began or ended.
pub const CORE_ANNOTATIONS: &'static [&'static str] = &[CLIENT_SEND,
                                                        CLIENT_RECV,
                                                        SERVER_SEND,
                                                        SERVER_RECV,
                                                        WIRE_SEND,
                                                        WIRE_RECV,
                                                        CLIENT_SEND_FRAGMENT,
                                                        CLIENT_RECV_FRAGMENT,
                                                        SERVER_SEND_FRAGMENT,
                                                        SERVER_RECV_FRAGMENT];

/// The domain portion of the URL or host header. Ex. "mybucket.s3.amazonaws.com"
///
/// Used to filter by host as opposed to ip address.
pub const HTTP_HOST: &'static str = "http.host";

/// The HTTP method, or verb, such as "GET" or "POST".
///
/// Used to filter against an http route.
pub const HTTP_METHOD: &'static str = "http.method";

/// The absolute http path, without any query parameters. Ex. "/objects/abcd-ff"
///
/// Used to filter against an http route, portably with zipkin v1.
pub const HTTP_PATH: &'static str = "http.path";

/// The entire URL, including the scheme, host and query parameters if available.
///
/// Combined with HTTP_METHOD, you can understand the fully-qualified request line.
/// Ex. "https://mybucket.s3.amazonaws.com/objects/abcd-ff?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Algorithm=AWS4-HMAC-SHA256..."
pub const HTTP_URL: &'static str = "http.url";

/// The HTTP status code, when not in 2xx range. Ex. "503"
///
/// Used to filter for error status.
pub const HTTP_STATUS_CODE: &'static str = "http.status_code";

/// The size of the non-empty HTTP request body, in bytes. Ex. "16384"
///
/// Large uploads can exceed limits or contribute directly to latency.
pub const HTTP_REQUEST_SIZE: &'static str = "http.request.size";

/// The size of the non-empty HTTP response body, in bytes. Ex. "16384"
///
/// Large downloads can exceed limits or contribute directly to latency.
pub const HTTP_RESPONSE_SIZE: &'static str = "http.response.size";

/// The query executed for SQL call.
///
/// Used to filter by SQL query.
/// Ex. "select * from customers where id = ?"
pub const SQL_QUERY: &'static str = "sql.query";
