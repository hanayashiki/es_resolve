
use tracing::Level;

pub fn with_tracing(f: fn() -> ()) {
    let collector = tracing_subscriber::fmt()
        // filter spans/events with level TRACE or higher.
        .with_max_level(Level::DEBUG)
        // build but do not install the subscriber.
        .finish();

    tracing::subscriber::with_default(collector, || {
        tracing::debug!("test tracing starts");
        f();
        tracing::debug!("test tracing ends");
    });
}
