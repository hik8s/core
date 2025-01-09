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

#[macro_export]
macro_rules! log_error_with_message {
    ($msg:expr, $e:expr) => {{
        let file_line = format!("{}:{}:{}", file!(), line!(), column!());
        let span = tracing::span!(tracing::Level::ERROR, "error", caller = file_line);
        let _enter = span.enter();
        tracing::error!("{}: {:?}", $msg, $e);
        $e
    }};
}

#[macro_export]
macro_rules! log_error_continue {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                log_error!(e);
                continue;
            }
        }
    };
}

#[macro_export]
macro_rules! log_error_break {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                log_error!(e);
                break;
            }
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    ($e:expr) => {{
        let file_line = format!("{}:{}:{}", file!(), line!(), column!());
        let span = tracing::span!(tracing::Level::WARN, "warn", caller = file_line);
        let _enter = span.enter();
        tracing::warn!("{:?}", $e);
        $e
    }};
}

#[macro_export]
macro_rules! log_warn_with_message {
    ($msg:expr, $e:expr) => {{
        let file_line = format!("{}:{}:{}", file!(), line!(), column!());
        let span = tracing::span!(tracing::Level::WARN, "warn", caller = file_line);
        let _enter = span.enter();
        tracing::warn!("{}: {:?}", $msg, $e);
        $e
    }};
}

#[macro_export]
macro_rules! log_warn_continue {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                let file_line = format!("{}:{}:{}", file!(), line!(), column!());
                let span = tracing::span!(tracing::Level::WARN, "warn", caller = file_line);
                let _enter = span.enter();
                tracing::warn!("{:?}", e);
                continue;
            }
        }
    };
}
