//! Verifies the cross-version metadata path: metadata must be reachable through
//! the shared `erris_meta::ReportMeta` trait, not only via
//! `downcast_ref::<ReportError>` (which fails across a version boundary because
//! each erris build has a distinct `TypeId` for `ReportError`).
//!
//! We can't build two real erris versions inside one test crate, so we
//! simulate the boundary: reading a report's location strictly through
//! `&dyn ReportMeta` is exactly what a *different* erris version would do.

use erris_meta::ReportMeta;

#[test]
fn metadata_readable_through_shared_trait() {
    let report = erris::report!("boundary error");

    // `&*report` derefs to ReportError. A foreign erris version only ever sees
    // it as `&dyn ReportMeta` (shared TypeId) — never as its own ReportError.
    let meta: &dyn ReportMeta = &*report;

    assert!(
        meta.report_location().is_some(),
        "location must be reachable via the shared trait across versions"
    );
    assert!(
        !meta.report_is_transparent(),
        "a message report is not transparent"
    );
}

#[test]
fn transparent_flag_through_shared_trait() {
    let report = erris::report!(); // transparent
    let meta: &dyn ReportMeta = &*report;
    assert!(meta.report_is_transparent());
}

#[test]
fn wrapper_branches_survive_through_shared_trait() {
    // A context wrapper has two children: cause + message. Both must be
    // reachable via report_branch(), which is the path a different erris
    // version uses. (Traversing via dyn Error::source would drop the message.)
    let wrapped = erris::report!("the cause").with_message("the context");
    let root: &dyn ReportMeta = &*wrapped;

    let (lhs, rhs) = root.report_branch();
    let lhs = lhs.map(|m| (m as &dyn std::error::Error).to_string());
    let rhs = rhs.map(|m| (m as &dyn std::error::Error).to_string());

    assert_eq!(lhs.as_deref(), Some("the cause"));
    assert_eq!(rhs.as_deref(), Some("the context"), "message branch must not be lost");
}

#[test]
fn upcast_meta_to_error_is_stable() {
    // dyn ReportMeta -> dyn Error (supertrait upcast, stable since 1.86).
    let report = erris::report!("x");
    let meta: &dyn ReportMeta = &*report;
    let err: &dyn std::error::Error = meta;
    assert_eq!(err.to_string(), "x");
}
