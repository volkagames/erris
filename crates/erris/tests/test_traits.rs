//! Marker traits on `Report` and the `?`-operator conversion from a std error.

use erris::*;

// Each bound is a separate compile-time assertion so a regression names the
// exact trait that `Report` stopped satisfying.

#[test]
fn report_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<Report>();
}

#[test]
fn report_is_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<Report>();
}

#[test]
fn report_is_unpin() {
    fn assert_unpin<T: Unpin + 'static>() {}
    assert_unpin::<Report>();
}

#[test]
fn report_is_debug() {
    fn assert_debug<T: std::fmt::Debug>() {}
    assert_debug::<Report>();
}

#[test]
fn question_mark_converts_std_error_into_report() {
    fn fails() -> std::result::Result<(), std::io::Error> {
        Err(std::io::Error::other("oh no!"))
    }

    fn wraps() -> erris::Result<()> {
        fails()?; // io::Error -> Report via From
        Ok(())
    }

    let err = wraps().unwrap_err();
    assert_eq!(err.to_string(), "oh no!");
}
