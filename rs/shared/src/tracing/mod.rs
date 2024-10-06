pub mod setup;

#[macro_export]
macro_rules! log_error {
    ($e:expr) => {{
        let file_line = format!("{}:{}:{}", file!(), line!(), column!());
        let span = tracing::span!(tracing::Level::ERROR, "error", caller = file_line);
        let _enter = span.enter();
        tracing::error!("{:?}", $e);
        $e
    }};
}
