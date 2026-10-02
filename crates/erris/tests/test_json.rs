#![cfg(feature = "to_json")]
mod common;
use rusty_fork::rusty_fork_test;

/// JSON snapshot of the same deeply nested error as the Debug snapshot in
/// `test_formatting`. `message` and `cause` are asserted verbatim; `location`
/// is checked by count so the test survives line-number shifts. Runs without a
/// subscriber, so `spantrace` is empty.
#[test]
fn json_snapshot_of_deeply_nested_error() {
    use common::{Io, TestError};
    use erris::report;

    let leaf = report!(Io("disk gone"));
    let with_std = leaf.with_err(TestError);
    let nested = report!(with_std);
    let ctx = nested.with_message("loading config");
    let ctx2 = ctx.with_report(report!("stage: init"));
    let top = ctx2.with_message("startup failed");

    let json = top.to_json_error();

    assert_eq!(json.message, "startup failed");
    assert_eq!(
        json.cause,
        vec![
            "stage: init",
            "loading config",
            "TestError",
            "io: disk gone"
        ]
    );
    // `ctx.with_report(report!(..))` builds two reports on one line, in different
    // columns: `keep-duplicate-location` lists both, the default collapses them.
    let expected = if cfg!(feature = "keep-duplicate-location") {
        7
    } else {
        6
    };
    assert_eq!(
        json.location.len(),
        expected,
        "one location per tracked node"
    );
    assert!(json.location.iter().all(|l| l.contains("test_json.rs")));
    assert!(json.spantrace.is_empty(), "no subscriber installed");
}

#[test]
fn to_json_safe_returns_valid_json_on_the_happy_path() {
    use erris::report;

    let report = report!("boom").with_message("context");
    let safe = report.to_json_safe();

    // to_json_safe falls back to Display only if serialization fails; a normal
    // report always serializes, so it must equal the fallible to_json output and
    // parse as a JSON object carrying the message.
    assert_eq!(safe, report.to_json().unwrap());
    let value: serde_json::Value = serde_json::from_str(&safe).expect("valid json");
    assert_eq!(value["message"], "context");
}

#[test]
fn to_json_pretty_is_multiline_and_matches_the_value() {
    use erris::report;

    let report = report!("boom").with_message("context");
    let pretty = report.to_json_pretty().unwrap();

    // pretty output is indented across lines and round-trips to the same value
    // as the compact form.
    assert!(pretty.contains('\n'));
    let from_pretty: serde_json::Value = serde_json::from_str(&pretty).unwrap();
    assert_eq!(from_pretty, report.to_json_value());
    assert_eq!(from_pretty["message"], "context");
}

#[test]
fn to_json_value_shapes_the_object_without_a_subscriber() {
    use erris::{JsonError, report};

    let report = report!("root").with_message("outer");
    let value = report.to_json_value();
    let obj = value.as_object().expect("json object");

    assert_eq!(obj["message"], "outer");
    let cause = obj["cause"].as_array().expect("cause array");
    assert_eq!(cause, &vec![serde_json::Value::from("root")]);
    assert!(
        !obj["location"]
            .as_array()
            .expect("location array")
            .is_empty()
    );
    // no subscriber installed: spantrace is present but empty
    assert!(
        obj["spantrace"]
            .as_array()
            .expect("spantrace array")
            .is_empty()
    );

    // to_json_error round-trips through serde back into JsonError.
    let json_str = report.to_json().unwrap();
    let parsed: JsonError = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.message, "outer");
    assert_eq!(parsed.cause, vec!["root".to_string()]);
}

fn install_logger() {
    use tracing_log::LogTracer;
    use tracing_log::log::LevelFilter;
    use tracing_subscriber::prelude::*;

    LogTracer::builder()
        .ignore_crate("rustls")
        .with_max_level(LevelFilter::Debug)
        .init()
        .unwrap();

    let subscriber = std::sync::Arc::new(
        tracing_subscriber::Registry::default()
            .with(tracing_error::ErrorLayer::default())
            .with(erris::tracing_fields::Layer::default()),
    );

    erris::tracing_fields::set_global_subscriber(subscriber.clone()).unwrap();
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

rusty_fork_test! {
    #[test]
    #[cfg(feature = "to_json")]
    fn test_to_json_print() {
        install_logger();

        #[tracing::instrument]
        fn some_fn(value: i32) -> erris::Report {
            let err = erris::report!("demo error eba").with_message("some description");

            {
                // Previously fetched via error_generic_member_access; the
                // SpanTrace is now read directly through ReportMeta.
                use erris_meta::ReportMeta;
                let trace = (*err).report_spantrace();
                assert!(trace.is_some());
                assert_eq!(
                    trace.unwrap().status(),
                    tracing_error::SpanTraceStatus::CAPTURED
                );
            }

            tracing::info!("test message with span");

            err
        }

        let _span = tracing::info_span!("A span", hello = "world", aaa = "bbb").entered();

        let err = some_fn(42);

        let msg = format!("{err:?}");
        println!("msg: {msg}");
        assert!(msg.as_str().contains("value=42"));

        let json = err.to_json_pretty().unwrap();
        println!("{json}");
        assert!(json.contains("\"value\": 42"));
        assert!(json.contains("\"hello\": \"world\""));
        assert!(json.contains("\"aaa\": \"bbb\""));
    }

    #[test]
    #[cfg(feature = "to_json")]
    #[cfg(feature = "spantrace")]
    fn test_to_json_nested() {
        install_logger();

        let _span = tracing::info_span!("A span", hello = "world", aaa = "bbb").entered();

        let err = erris::report!("some error message");
        let other_err = erris::report!("other message");
        let err = err.with_report(other_err);
        let err = err.with_report(erris::report!("more message"));
        let err = err.with_message("from message");
        println!("{err:?}");
        println!("{err:#?}");

        let inner = err.unwrap_ref::<erris::ReportError>();
        assert!(inner.is_some());

        // Location is now exposed through ReportMeta instead of provide().
        use erris_meta::ReportMeta;
        let location = inner.unwrap().report_location();
        assert!(location.is_some());

        println!("to json example:");
        {
            let json = err.to_json_pretty().expect("serialize ok");
            println!("{json}");
            let json_error : erris::JsonError = serde_json::from_str(&json).expect("deserialize ok");
            assert!(!json_error.message.is_empty());
        }

        let json = err.to_json_value();
        let json_object = json.as_object().expect("expected object");
        {
            let json_cause = json_object.get("cause").expect("expect cause field");
            let json_cause = json_cause.as_array().expect("expect cause is a array");
            // top message ("from message") is the `message` field, not a cause
            assert_eq!(
                json_cause,
                &vec!["more message", "other message", "some error message"]
            );
        }
        {
            let json_location = json_object.get("location").expect("expect location field");
            let json_location = json_location.as_array().expect("expect location is a array");
            // `err.with_report(erris::report!(..))` builds two reports on one line, in
            // different columns: `keep-duplicate-location` lists both.
            let expected = if cfg!(feature = "keep-duplicate-location") { 6 } else { 5 };
            assert_eq!(json_location.len(), expected);
        }
        {
            let json_spantrace = json_object.get("spantrace").expect("expect spantrace field");
            let json_spantrace = json_spantrace.as_array().expect("expect spantrace is a array");
            assert!(!json_spantrace.is_empty());
        }


    }
}
