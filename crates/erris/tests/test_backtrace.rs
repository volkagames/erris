#![cfg(feature = "backtrace")]

#[test]
fn test_backtrace() {
    let report = erris::report!("test");
    assert!(report.backtrace().is_some());

    let report2 = report.with_message("test2");
    assert!(report2.backtrace().is_some()); // nested backtrace
    assert!(report2.inner_backtrace().is_none()); // only one backtrace crated

    let report = erris::report!(std::io::Error::other("test"));
    assert!(report.backtrace().is_some());
}
