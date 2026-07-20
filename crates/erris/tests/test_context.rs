//! Adding context on the `Result` / `Option` side: `wrap_report`,
//! `ok_or_report`, and `track`.

mod common;
use common::TestError;
use erris::prelude::*;
use erris::report;

#[test]
fn ok_or_report_on_option() {
    assert!(Some(true).ok_or_report("unused").is_ok());

    let err = None::<bool>.ok_or_report("expected a value").unwrap_err();
    assert_eq!(err.to_string(), "expected a value");
}

#[test]
fn ok_or_report_on_result_keeps_error_and_adds_message() {
    let res: std::result::Result<(), _> = Err(std::io::Error::other("io boom"));
    let err = res.ok_or_report("while loading").unwrap_err();

    // the added message is the top-level display; the original error is a cause
    assert_eq!(err.to_string(), "while loading");
    assert!(format!("{err:?}").contains("io boom"));
}

#[test]
fn wrap_report_on_result_adds_context() {
    fn validate() -> Result<(), Report> {
        Err(report!("oh noooo!"))
    }
    let err = validate().wrap_report("wrapped error").unwrap_err();
    assert_eq!(err.to_string(), "wrapped error");
    assert!(format!("{err:?}").contains("oh noooo!"));
}

#[test]
fn wrap_report_on_std_error_result() {
    let res: erris::Result<()> = Err(std::io::Error::other("oh no!")).wrap_report("oops");
    let err = res.unwrap_err();
    assert_eq!(err.to_string(), "oops");
    assert!(format!("{err:?}").contains("oh no!"));
}

#[test]
fn wrap_report_on_boxed_dyn_error_result() {
    fn produce() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Err("boxed boom")?;
        Ok(())
    }
    let err = produce().wrap_report("wrapped").unwrap_err();
    assert_eq!(err.to_string(), "wrapped");
    assert!(format!("{err:?}").contains("boxed boom"));
}

#[test]
fn wrap_report_with_is_lazy() {
    // the closure is only evaluated on the Err path
    let ok: erris::Result<i32> = Ok::<i32, Report>(1).wrap_report_with(|| -> Report {
        panic!("must not run on Ok")
    });
    assert_eq!(ok.unwrap(), 1);
}

#[test]
fn track_preserves_the_error() {
    let res: std::result::Result<(), _> = Err(TestError);
    let err = res.track().unwrap_err();
    assert_eq!(err.to_string(), "TestError");
}

#[test]
fn ok_paths_pass_the_value_through_unchanged() {
    // The Ok arm of every extension trait must return the value untouched.
    assert_eq!(Ok::<_, TestError>(1).track().unwrap(), 1);
    assert_eq!(Ok::<_, TestError>(2).wrap_report("unused").unwrap(), 2);
    assert_eq!(Ok::<_, TestError>(3).ok_or_report("unused").unwrap(), 3);

    let boxed: Result<i32, Box<dyn std::error::Error + Send + Sync + 'static>> = Ok(4);
    assert_eq!(boxed.wrap_report("unused").unwrap(), 4);
}

#[test]
fn ok_or_report_with_builds_lazily_on_the_empty_case() {
    // Some / Ok short-circuit without calling the closure.
    assert_eq!(Some(1).ok_or_report_with(|| report!("unused")).unwrap(), 1);
    assert_eq!(
        Ok::<_, TestError>(2).ok_or_report_with(|| report!("unused")).unwrap(),
        2
    );

    // None builds the report from the closure.
    let err = None::<i32>.ok_or_report_with(|| report!("none built")).unwrap_err();
    assert_eq!(err.to_string(), "none built");

    // Err keeps the original error as a cause under the closure-built report.
    let err = Err::<i32, _>(TestError)
        .ok_or_report_with(|| report!("err context"))
        .unwrap_err();
    assert_eq!(err.to_string(), "err context");
    assert!(format!("{err:?}").contains("TestError"));
}

#[test]
fn wrap_report_with_wraps_on_the_err_case() {
    let err = Err::<(), _>(TestError)
        .wrap_report_with(|| report!("lazy context"))
        .unwrap_err();
    assert_eq!(err.to_string(), "lazy context");
    assert!(format!("{err:?}").contains("TestError"));
}

#[test]
fn wrap_box_report_with_wraps_a_boxed_error() {
    fn produce() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Err("boxed boom")?;
        Ok(())
    }
    let err = produce()
        .wrap_report_with(|| report!("lazy boxed context"))
        .unwrap_err();
    assert_eq!(err.to_string(), "lazy boxed context");
    assert!(format!("{err:?}").contains("boxed boom"));
}
