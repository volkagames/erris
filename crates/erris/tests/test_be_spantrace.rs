//! The comparison macros enter a span carrying both operands before building
//! the report, so the values reach the span trace even when the caller supplied
//! its own message — under an ordinary `INFO` subscriber, not just an unfiltered
//! one.
#![cfg(feature = "spantrace")]

use rusty_fork::rusty_fork_test;

/// An `INFO`-filtered subscriber — what an application actually runs. A span
/// below that level is never recorded, so the operands would be lost.
fn install_subscriber() {
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::prelude::*;

    let subscriber = tracing_subscriber::Registry::default()
        .with(LevelFilter::INFO)
        .with(tracing_error::ErrorLayer::default());
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

rusty_fork_test! {
    #[test]
    fn comparisons_record_their_operands_in_a_span() {
        use erris::be;

        install_subscriber();

        let default_message = be::eq!(1, 2).unwrap_err();
        let trace = default_message.spantrace().to_string();
        assert!(trace.contains("be::eq"), "span name missing: {trace}");
        assert!(trace.contains("left=1"), "left operand missing: {trace}");
        assert!(trace.contains("right=2"), "right operand missing: {trace}");

        // Regression: the span used to be skipped whenever a message was given,
        // so the operands were lost exactly when the message hid them.
        let own_message = be::ne!(2, 2, "still equal").unwrap_err();
        let trace = own_message.spantrace().to_string();
        assert_eq!(own_message.to_string(), "still equal");
        assert!(trace.contains("be::ne"), "span name missing: {trace}");
        assert!(trace.contains("left=2"), "left operand missing: {trace}");
        assert!(trace.contains("right=2"), "right operand missing: {trace}");
    }
}
