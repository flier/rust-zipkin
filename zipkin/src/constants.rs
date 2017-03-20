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
