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

    fn wraps() -> std::result::Result<(), Report> {
        fails()?; // io::Error -> Report via From
        Ok(())
    }

    let err = wraps().unwrap_err();
    assert_eq!(err.to_string(), "oh no!");
}

/// `into_std()` reads the same with and without `tracked`: the `IntoStd`
/// identity on a std result, the inherent method on a `TrackedResult`.
#[test]
fn into_std_in_every_mode() {
    fn load(ok: bool) -> erris::Result<u32> {
        if ok {
            erris::Result::Ok(1)
        } else {
            erris::Result::Err(erris::report!("boom"))
        }
    }

    let ok: std::result::Result<u32, String> = load(true).into_std().map_err(|e| e.to_string());
    assert_eq!(ok, std::result::Result::Ok(1));
    let failed: std::result::Result<u32, String> =
        load(false).into_std().map_err(|e| e.to_string());
    assert_eq!(failed, std::result::Result::Err("boom".to_owned()));
}
