//! Building a `Report` from the various sources the `report!` macro and the
//! `From` / `IntoReport` conversions accept: messages, std errors, boxed dyn
//! errors, other reports, and Arc-shared reports.

mod common;
use common::TestError;
use erris::{BoxIntoReport, IntoReport, Report, ReportError, ReportType, report};
use std::borrow::Cow;
use std::sync::Arc;

#[test]
fn report_macro_from_message_kinds() {
    let _: Report = report!("literal");
    let _: Report = report!("owned".to_string());
    let _: Report = report!(Cow::Borrowed("cow"));
}

#[test]
fn report_macro_from_error_and_report() {
    let _: Report = report!(TestError);
    let _: Report = report!(report!("inner"));
    let _: Report = report!(Box::new(std::io::Error::other("boxed sized")));
}

#[test]
fn report_macro_transparent_is_empty() {
    assert_eq!(report!().to_string(), "");
}

#[test]
fn from_std_error_is_transparent() {
    let report: Report = Report::from(std::io::Error::other("io error"));
    assert!(report.is_transparent_error());
}

#[test]
fn from_dyn_boxed_is_transparent() {
    let report = Report::from_dyn_boxed("string error".to_string().into());
    assert!(report.is_transparent_error());
}

#[test]
fn into_report_on_std_error() {
    let report: Report = TestError.into_report();
    assert_eq!(report.to_string(), "TestError");
}

#[test]
fn into_report_on_report_is_identity_message() {
    let report: Report = report!("keep me").into_report();
    assert_eq!(report.to_string(), "keep me");
}

#[test]
fn with_dyn_boxed_wraps_self_under_the_boxed_error() {
    // with_dyn_boxed mirrors with_err: the *boxed* error becomes the new top
    // (its Display), and `self` is pushed underneath as the cause.
    let boxed: Box<dyn std::error::Error + Send + Sync + 'static> = "new top".into();
    let report = report!("the cause").with_dyn_boxed(boxed);

    assert_eq!(report.to_string(), "new top");
    assert!(format!("{report:?}").contains("the cause"));
}

#[test]
fn from_arc_report_displays_transparently() {
    // report!(Arc<Report>) dispatches through FromArc; its Display and unwrap
    // path must see through the Arc link.
    let shared = Arc::new(report!(TestError));
    let report: Report = report!(shared);
    assert_eq!(report.to_string(), "TestError");
}

#[test]
fn reuse_rewraps_an_existing_report_error() {
    // Converting a value that is *already* a ReportError hits the transmute
    // fast path in reuse_report_error instead of boxing it afresh. Building a
    // ReportError directly (not a Report) is the only way to reach it.
    let inner = ReportError::new(ReportType::Message("reused".into()));
    let report: Report = inner.into_report();
    assert_eq!(report.to_string(), "reused");
    // The recovered ReportError is the same value, reachable via unwrap_ref.
    assert!(report.unwrap_ref::<ReportError>().is_some());
}

#[test]
fn report_error_new_builds_a_message_error() {
    let err = ReportError::new(ReportType::Message("built directly".into()));
    assert_eq!(err.to_string(), "built directly");
    assert!(!err.is_transparent_error());
    // NB: unlike the Report::from_* constructors, ReportError::new is not
    // #[track_caller], so its recorded location is report.rs (the new_track!
    // site), not this caller. Pin that so the divergence is intentional.
    assert!(err.location().file().ends_with("report.rs"));
}

#[test]
fn report_converts_into_boxed_dyn_error() {
    let report = report!("into box");

    let boxed: Box<dyn std::error::Error + Send + Sync + 'static> = report.into();
    assert_eq!(boxed.to_string(), "into box");

    let report = report!("into box no send");
    let boxed: Box<dyn std::error::Error + 'static> = report.into();
    assert_eq!(boxed.to_string(), "into box no send");
}

#[test]
fn boxed_dyn_error_paths() {
    fn make_dyn_error() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Err("oh noooo!")?;
        Ok(())
    }

    let dyn_error = make_dyn_error().err().unwrap();
    let report = Report::from_dyn_boxed(dyn_error);
    assert_eq!(report.to_string(), "oh noooo!");

    let dyn_error = make_dyn_error().err().unwrap();
    let report: Report = report!(dyn_error);
    assert_eq!(report.to_string(), "oh noooo!");

    let dyn_error = make_dyn_error().err().unwrap();
    let report: Report = dyn_error.into_report();
    assert_eq!(report.to_string(), "oh noooo!");
}
