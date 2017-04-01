#[macro_export]
macro_rules! annotate {
    ($span:ident, $value:expr) => {
        if $span.used() {
            $span.annotate($value, None);
        }
    };
    ($span:ident, $value:expr, endpoint => $endpoint:expr) => {
        if $span.used() {
            $span.annotate($value, $endpoint);
        }
    };
    ($span:ident, $key:expr, $value:expr) => {
        if $span.used() {
            $span.binary_annotate($key, $value, None);
        }
    };
    ($span:ident, $key:expr, $value:expr, endpoint => $endpoint:expr) => {
        if $span.used() {
            $span.binary_annotate($key, $value, $endpoint);
        }
    };
}
