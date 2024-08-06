use std::{env::var, sync::Once};

use tracing_subscriber::{EnvFilter, FmtSubscriber};

static INIT: Once = Once::new();

pub fn setup_tracing() {
    // call_once allows to initialize the logger from multiple tests running in separated threads
    INIT.call_once(|| {
        let filter = EnvFilter::new(var("RUST_LOG").unwrap_or_else(|_| String::from("info")));

        let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    });
}
