use std::{env::var, sync::Once};
use tracing_subscriber::{fmt, EnvFilter, FmtSubscriber};

static TRACING_INIT: Once = Once::new();

pub fn setup_tracing() {
    // ensures that the subscriber is only initialized once for all threads
    TRACING_INIT.call_once(|| {
        let filter = EnvFilter::new(var("RUST_LOG").unwrap_or_else(|_| String::from("info")));

        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .fmt_fields(fmt::format::DefaultFields::new())
            .event_format(fmt::format().compact().with_line_number(true))
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    });
}
