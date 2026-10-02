//! `tracked_prelude`: globbing `erris::prelude` is enough to substitute the
//! tracking `Result`, `Ok` and `Err` for std's. This file imports nothing else
//! on purpose.
#![cfg(feature = "tracked_prelude")]

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
fn leaf() -> Result<u32> {
    Err(erris::report!("boom"))
}

#[test]
fn prelude_result_tracks_on_question_mark() {
    const HOP: u32 = line!() + 2;
    fn hop() -> Result<u32> {
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
