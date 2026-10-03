//! `tracked`: globbing `erris::prelude` is enough to substitute the tracking
//! `Ok` and `Err` for std's, while a bare `Result` stays std's. This file
//! imports nothing else on purpose.
#![cfg(feature = "tracked")]

use erris::prelude::*;

/// Lines of the report's own links, outermost first.
fn lines(report: &Report) -> Vec<u32> {
    report
        .chain()
        .filter_map(|link| link.location())
        .map(|location| location.line())
        .collect()
}

const LEAF: u32 = line!() + 2;
fn leaf() -> erris::Result<u32> {
    Err(erris::report!("boom"))
}

#[test]
fn prelude_result_tracks_on_question_mark() {
    const HOP: u32 = line!() + 2;
    fn hop() -> erris::Result<u32> {
        let n = leaf()?;
        Ok(n)
    }

    assert_eq!(lines(&hop().unwrap_err()), [HOP, LEAF]);
}

#[test]
fn prelude_names_are_the_tracked_ones() {
    let ok: erris::TrackedResult<u32> = Ok(1);
    assert_eq!(ok.ok(), Some(1));

    let err: erris::TrackedResult<u32> = leaf();
    assert!(matches!(err, Err(_)));
}

#[test]
fn bare_result_stays_std() {
    // Only `Ok`/`Err` come from the prelude; a std result is still written
    // `Result<T, E>`, with its variants spelled out.
    let std: Result<u32, String> = std::result::Result::Ok(1);
    assert_eq!(std.ok(), Some(1));
}

#[test]
fn into_std_hands_off_to_std_apis() {
    let failed: std::result::Result<u32, String> = leaf().into_std().map_err(|e| e.to_string());
    assert_eq!(failed, std::result::Result::Err("boom".to_owned()));
}
