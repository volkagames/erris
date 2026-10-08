//! Recovering typed errors out of a `Report`: `unwrap_ref` / `unwrap_mut` /
//! `unwrap_recursive`, and the `AsRef<dyn Error>` views.

mod common;
use common::TestError;
use erris::*;
use std::sync::Arc;

#[test]
fn unwrap_ref_finds_a_wrapped_std_error() {
    let report = report!(TestError);
    assert!(report.unwrap_ref::<TestError>().is_some());
}

#[test]
fn unwrap_ref_traverses_wrapper_branches() {
    let report = report!(TestError).with_message("context");
    // reachable through the cause branch of the wrapper
    assert!(report.unwrap_ref::<TestError>().is_some());
}

#[test]
fn unwrap_recursive_traverses_nested_reports() {
    let report = report!(report!(TestError));
    assert!(report.unwrap_recursive::<TestError>().is_some());
}

#[test]
fn unwrap_mut_does_not_traverse_arc_report() {
    // An ArcReport link is shared, so unwrap_mut cannot reach through it while
    // unwrap_ref / unwrap_recursive can. Pins that documented divergence.
    let mut report = report!(Arc::new(report!(TestError)));

    assert!(report.unwrap_ref::<TestError>().is_some());
    assert!(report.unwrap_recursive::<TestError>().is_some());
    assert!(report.unwrap_mut::<TestError>().is_none());
}

#[test]
fn unwrap_follows_the_source_chain_of_a_boxed_error() {
    // A std error whose own `source()` points at another error. The Debug/JSON
    // chain follows `source()`, so `unwrap_ref` / `unwrap_recursive` must reach
    // the inner type too, not just the outer one.
    #[derive(Debug)]
    struct Inner;
    impl std::fmt::Display for Inner {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("inner")
        }
    }
    impl std::error::Error for Inner {}

    #[derive(Debug)]
    struct Outer(Inner);
    impl std::fmt::Display for Outer {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("outer")
        }
    }
    impl std::error::Error for Outer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    let report = report!(Outer(Inner));

    assert!(format!("{report:?}").contains("inner"));
    assert!(report.unwrap_ref::<Outer>().is_some());
    assert!(report.unwrap_ref::<Inner>().is_some());
    assert!(report.unwrap_recursive::<Inner>().is_some());
}

#[test]
fn unwrap_mut_traverses_wrapper_and_nested_report() {
    // The mutable path must descend both a context wrapper's cause branch and a
    // nested report, mirroring unwrap_ref (minus the ArcReport case above).
    let mut wrapped = report!(TestError).with_message("context");
    assert!(
        wrapped.unwrap_mut::<TestError>().is_some(),
        "through wrapper cause"
    );

    let mut nested = report!(report!(TestError));
    assert!(
        nested.unwrap_mut::<TestError>().is_some(),
        "through nested report"
    );
}

#[test]
fn unwrap_mut_returns_the_report_error_itself() {
    // T == ReportError hits the identity fast path (TypeId self-match).
    let mut report = report!("boom");
    assert!(report.unwrap_mut::<ReportError>().is_some());
}

#[test]
fn unwrap_recursive_descends_through_arc_report() {
    // unwrap_recursive follows an ArcReport link (unlike unwrap_mut). Nest the
    // Arc under another report so the recursive descent is exercised.
    let report = report!(report!(Arc::new(report!(TestError))));
    assert!(report.unwrap_recursive::<TestError>().is_some());
}

#[test]
fn unwrap_ref_returns_none_for_absent_type() {
    // A message-only report has no wrapped std error to recover.
    #[derive(Debug)]
    struct NotThere;
    impl std::fmt::Display for NotThere {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("nope")
        }
    }
    impl std::error::Error for NotThere {}

    let report = report!("just a message");
    assert!(report.unwrap_ref::<NotThere>().is_none());
    assert!(report.unwrap_recursive::<NotThere>().is_none());
}

#[test]
fn as_ref_yields_error_trait_objects() {
    let report = report!("some error message");

    let error_ref: &(dyn std::error::Error + 'static) = report.as_ref();
    assert_eq!(error_ref.to_string(), "some error message");

    let send_sync_ref: &(dyn std::error::Error + Send + Sync + 'static) = report.as_ref();
    assert_eq!(send_sync_ref.to_string(), "some error message");
}

#[test]
fn boxed_dyn_error_downcasts_to_report_error() {
    // The erased payload is the ReportError itself, not a Box<ReportError>.
    let boxed: Box<dyn std::error::Error + Send + Sync> = report!("boom").into();
    assert!(boxed.downcast_ref::<ReportError>().is_some());
    assert!(boxed.downcast_ref::<Box<ReportError>>().is_none());

    let boxed: Box<dyn std::error::Error> = report!("boom").into();
    assert!(boxed.downcast_ref::<ReportError>().is_some());
}
