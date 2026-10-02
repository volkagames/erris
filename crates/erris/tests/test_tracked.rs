//! `erris::tracked::Result`: `?` adds a location frame on every hop, including
//! the hops across the std `Result` boundary in both directions.
#![cfg(feature = "tracked")]

use erris::prelude::*;
use erris::tracked::{Err, Ok, Result};
use erris::{be, report};
use std::result::Result as StdResult;

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
    Err(report!("boom"))
}

/// A typed `Ok`: a bare `Ok(n)` leaves the error parameter open, as std's does.
fn ok(n: u32) -> Result<u32> {
    Ok(n)
}

const MID: u32 = line!() + 2;
fn mid() -> Result<u32> {
    let n = leaf()?;
    Ok(n)
}

#[test]
fn question_mark_tracks_every_hop() {
    const TOP: u32 = line!() + 2;
    fn top() -> Result<u32> {
        let n = mid()?;
        Ok(n)
    }

    let report = top().err().expect("an error");
    assert_eq!(lines(&report), [TOP, MID, LEAF]);
    assert_eq!(report.to_string(), "boom");
}

#[test]
fn foreign_error_is_located_once() {
    const PARSE: u32 = line!() + 2;
    fn parse(s: &str) -> Result<u32> {
        let n: u32 = s.parse()?;
        Ok(n)
    }

    assert_eq!(parse("7").ok(), Some(7));
    let report = parse("x").err().expect("an error");
    assert_eq!(lines(&report), [PARSE]);
}

#[test]
fn std_result_with_a_report_is_tracked() {
    const STD_LEAF: u32 = line!() + 2;
    fn std_leaf() -> StdResult<u32, Report> {
        StdResult::Err(report!("boom"))
    }
    const HOP: u32 = line!() + 2;
    fn hop() -> Result<u32> {
        let n = std_leaf()?;
        Ok(n)
    }

    let report = hop().err().expect("an error");
    assert_eq!(lines(&report), [HOP, STD_LEAF]);
}

#[derive(Debug)]
struct AppError(Report);

impl From<Report> for AppError {
    fn from(report: Report) -> Self {
        AppError(report)
    }
}

#[test]
fn boundary_into_a_std_result_tracks() {
    // The shape of a handler whose signature a foreign trait dictates.
    const HANDLER: u32 = line!() + 2;
    fn handler() -> StdResult<u32, AppError> {
        let n = mid()?;
        StdResult::Ok(n)
    }

    let AppError(report) = handler().expect_err("an error");
    assert_eq!(lines(&report), [HANDLER, MID, LEAF]);
}

/// With `keep-duplicate-location` a frame that differs from the next one by
/// column alone is kept; without it `?` does not add such a frame.
fn with_same_line_frame(line: u32, rest: &[u32]) -> Vec<u32> {
    let mut lines = Vec::new();
    if cfg!(feature = "keep-duplicate-location") {
        lines.push(line);
    }
    lines.extend_from_slice(rest);
    lines
}

#[test]
fn question_mark_records_its_own_line_once() {
    // A report already located on the `?` line gets no second frame.
    const CHECK: u32 = line!() + 2;
    fn checked() -> Result<u32> {
        let n = be::some!(None::<u32>, "required")?;
        Ok(n)
    }
    assert_eq!(lines(&checked().unwrap_err()), [CHECK]);

    // The wrapper and its message are both located on the `wrap_report` line.
    const WRAP: u32 = line!() + 2;
    fn wrapped() -> Result<u32> {
        let n = leaf().wrap_report("context")?;
        Ok(n)
    }
    assert_eq!(
        lines(&wrapped().unwrap_err()),
        with_same_line_frame(WRAP, &[WRAP, WRAP, LEAF])
    );

    const TRACK: u32 = line!() + 3;
    fn tracked_by_hand() -> Result<u32> {
        #[allow(deprecated)]
        let n = leaf().track()?;
        Ok(n)
    }
    assert_eq!(
        lines(&tracked_by_hand().unwrap_err()),
        with_same_line_frame(TRACK, &[TRACK, LEAF])
    );
}

#[test]
fn question_mark_keeps_a_frame_from_another_line() {
    // `?` is located at the start of its expression, the wrapper one line below.
    const FIRST: u32 = line!() + 3;
    #[rustfmt::skip]
    fn wrapped_over_two_lines() -> Result<u32> {
        let n = leaf()
            .wrap_report("context")?;
        Ok(n)
    }

    assert_eq!(
        lines(&wrapped_over_two_lines().unwrap_err()),
        [FIRST, FIRST + 1, FIRST + 1, LEAF]
    );
}

/// Not a `std::error::Error`: a report is built from it by its own `From` impl.
struct Custom;

const CUSTOM_FROM: u32 = line!() + 3;
impl From<Custom> for Report {
    fn from(_: Custom) -> Report {
        report!("from custom")
    }
}

#[test]
fn question_mark_accepts_an_error_with_its_own_conversion() {
    const HOP: u32 = line!() + 2;
    fn hop() -> Result<u32> {
        let n = StdResult::<u32, Custom>::Err(Custom)?;
        Ok(n)
    }

    let report = hop().unwrap_err();
    assert_eq!(report.to_string(), "from custom");
    assert_eq!(lines(&report), [HOP, CUSTOM_FROM]);
}

#[test]
fn question_mark_tracks_inside_a_closure() {
    const CLOSURE: u32 = line!() + 2;
    let hop = || -> Result<u32> {
        let n = mid()?;
        Ok(n)
    };

    assert_eq!(lines(&hop().unwrap_err()), [CLOSURE, MID, LEAF]);
}

#[test]
fn question_mark_tracks_inside_an_async_fn() {
    const ASYNC_LEAF: u32 = line!() + 2;
    async fn async_leaf() -> Result<u32> {
        Err(report!("boom"))
    }
    const ASYNC_HOP: u32 = line!() + 2;
    async fn async_hop() -> Result<u32> {
        let n = async_leaf().await?;
        Ok(n)
    }

    let mut future = std::pin::pin!(async_hop());
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    let std::task::Poll::Ready(result) = future.as_mut().poll(&mut context) else {
        panic!("nothing in the future suspends");
    };
    assert_eq!(lines(&result.unwrap_err()), [ASYNC_HOP, ASYNC_LEAF]);
}

#[test]
fn a_hop_without_question_mark_adds_no_frame() {
    // Tracking is tied to `?`: handing the value on leaves the chain as it is.
    fn passthrough() -> Result<u32> {
        mid()
    }

    assert_eq!(lines(&passthrough().unwrap_err()), [MID, LEAF]);
}

#[test]
fn success_is_not_tracked() {
    fn ok_leaf() -> Result<u32> {
        Ok(1)
    }
    fn hop() -> Result<u32> {
        let n = ok_leaf()?;
        Ok(n + 1)
    }

    assert_eq!(hop().ok(), Some(2));
}

#[test]
fn map_and_map_err_stay_tracked() {
    let doubled: Result<u32> = Ok(2).map(|n| n * 2);
    assert_eq!(doubled.ok(), Some(4));

    let with_context: Result<u32> = leaf().map_err(|report| report.with_message("context"));
    assert_eq!(with_context.unwrap_err().to_string(), "context");

    // A foreign error returned by the closure is converted back into a report.
    const MAPPED: u32 = line!() + 1;
    let replaced: Result<u32> = leaf().map_err(|_| std::io::Error::other("io"));
    let report = replaced.unwrap_err();
    assert_eq!(report.to_string(), "io");
    assert_eq!(lines(&report), [MAPPED]);
}

#[test]
fn collect_stops_at_the_first_error() {
    let all: Result<Vec<u32>> = [Ok(1), Ok(2)].into_iter().collect();
    assert_eq!(all.ok(), Some(vec![1, 2]));

    let failed: Result<Vec<u32>> = [Ok(1), leaf(), Ok(3)].into_iter().collect();
    assert!(failed.is_err());
}

#[test]
fn crate_root_result_is_the_tracked_type() {
    // With the feature on, `erris::Result` and `erris::TrackedResult` are one type.
    fn by_alias() -> erris::Result<u32> {
        mid()
    }
    fn by_name() -> erris::TrackedResult<u32> {
        by_alias()
    }

    assert!(by_name().is_err());
    let by_path: erris::Result<u32> = erris::Result::Ok(1);
    assert_eq!(by_path.ok(), Some(1));
}

/// Without `tracked_prelude` the prelude leaves std's names alone.
#[cfg(not(feature = "tracked_prelude"))]
mod prelude_alone {
    use erris::prelude::*;

    #[test]
    fn does_not_substitute_result() {
        let std: Result<u32, Report> = Ok(1);
        assert_eq!(std.ok(), Some(1));
    }
}

#[test]
fn erris_extensions_stay_tracked() {
    let wrapped: Result<u32> = leaf().wrap_report("context");
    assert_eq!(wrapped.unwrap_err().to_string(), "context");

    let lazy: Result<u32> = leaf().wrap_report_with(|| report!("lazy context"));
    assert_eq!(lazy.unwrap_err().to_string(), "lazy context");

    // Deprecated because `?` already tracks; the frame it adds is still pinned here.
    const TRACK: u32 = line!() + 2;
    #[allow(deprecated)]
    let tracked: Result<u32> = leaf().track();
    assert_eq!(lines(&tracked.unwrap_err()), [TRACK, LEAF]);
}

#[test]
fn be_macros_accept_and_return_tracked_results() {
    // `ok!` and `err!` take a `TrackedResult` as they take a std `Result`.
    assert_eq!(be::ok!(ok(1)).ok(), Some(1));
    let report = be::ok!(leaf(), "leaf must succeed").unwrap_err();
    assert_eq!(report.to_string(), "leaf must succeed");
    assert!(
        report
            .chain()
            .any(|link| link.as_error().to_string() == "boom"),
        "the original error stays in the chain"
    );
    assert_eq!(be::err!(leaf()).unwrap().to_string(), "boom");
    assert!(be::err!(ok(1)).is_err());

    // By reference the result stays in place and the macro hands back references.
    let failed = leaf();
    assert_eq!(be::err!(&failed).unwrap().to_string(), "boom");
    assert!(failed.is_err());

    // A std `Result` still goes in, and what comes out is the crate's `Result`.
    let std: StdResult<u32, std::io::Error> = StdResult::Ok(1);
    let checked: Result<u32> = be::ok!(std);
    assert_eq!(checked.ok(), Some(1));

    // ... so a macro fits a tail position as it is.
    fn tail(value: u32) -> Result<()> {
        be::non_zero!(value)
    }
    assert!(tail(1).is_ok());
    assert_eq!(tail(0).unwrap_err().to_string(), "expected non zero: value");
}

#[test]
fn erris_apis_work_in_a_tracked_function() {
    fn checked(value: Option<u32>) -> Result<u32> {
        let value = be::some!(value, "value is required")?;
        be::non_zero!(value)?;
        Ok(value)
    }

    assert_eq!(checked(Some(2)).ok(), Some(2));
    assert_eq!(checked(None).unwrap_err().to_string(), "value is required");
    assert_eq!(
        checked(Some(0)).unwrap_err().to_string(),
        "expected non zero: value"
    );

    // The extension methods on `Option` and std `Result` hand back the crate's
    // `Result`, so they fit a tail position as they are.
    const TAIL: u32 = line!() + 2;
    fn tail(value: Option<u32>) -> Result<u32> {
        value.ok_or_report("missing")
    }
    assert_eq!(tail(Some(1)).ok(), Some(1));
    assert_eq!(lines(&tail(None).unwrap_err()), [TAIL]);

    fn wrapped(text: &str) -> Result<u32> {
        text.parse::<u32>().wrap_report("not a number")
    }
    assert_eq!(wrapped("7").ok(), Some(7));
    assert_eq!(wrapped("x").unwrap_err().to_string(), "not a number");

    #[allow(deprecated)]
    fn tracked(text: &str) -> Result<u32> {
        // `track` on a std `Result` is not the deprecated one, but what it
        // returns is, so the chain below warns.
        text.parse::<u32>().track().track()
    }
    assert!(tracked("x").is_err());

    // What they return is matched with the substituted `Ok` / `Err`.
    match "x".parse::<u32>().wrap_report("context") {
        Ok(n) => panic!("unexpected {n}"),
        Err(report) => assert_eq!(report.to_string(), "context"),
    }

    // A std `Result<T, Report>` from elsewhere still converts, without a frame.
    const STD_TAIL: u32 = line!() + 2;
    fn std_tail() -> StdResult<u32, Report> {
        StdResult::Err(report!("boom"))
    }
    fn converted() -> Result<u32> {
        std_tail().into()
    }
    assert_eq!(lines(&converted().unwrap_err()), [STD_TAIL]);
}

#[test]
fn predicates_and_extractors() {
    assert!(ok(2).is_ok_and(|n| n == 2));
    assert!(leaf().is_err_and(|report| report.to_string() == "boom"));
    assert_eq!(ok(2).ok(), Some(2));
    assert_eq!(
        leaf().err().map(|report| report.to_string()),
        Some("boom".to_string())
    );
}

#[test]
fn map_family() {
    assert_eq!(ok(2).map_or(0, |n| n * 2), 4);
    assert_eq!(leaf().map_or(0, |n| n * 2), 0);
    assert_eq!(
        leaf().map_or_else(|report| report.to_string(), |n| n.to_string()),
        "boom"
    );
    assert_eq!(leaf().map_or_default(|n| n * 2), 0);

    let mut seen = 0;
    let kept = ok(3).inspect(|n| seen = *n);
    assert_eq!((kept.ok(), seen), (Some(3), 3));

    let mut message = String::new();
    let kept = leaf().inspect_err(|report| message = report.to_string());
    assert!(kept.is_err());
    assert_eq!(message, "boom");
}

#[test]
fn and_family_stays_tracked() {
    assert_eq!(ok(1).and(Ok("x")).ok(), Some("x"));
    assert!(leaf().and(Ok(1)).is_err());
    assert_eq!(ok(2).and_then(|n| Ok(n + 1)).ok(), Some(3));
    assert!(ok(2).and_then(|_| leaf()).is_err());
}

#[test]
fn or_family_stays_tracked() {
    assert_eq!(leaf().or(Ok(1)).ok(), Some(1));
    assert_eq!(Ok(7).or(leaf()).ok(), Some(7));
    assert_eq!(leaf().or_else(|_| Ok(2)).ok(), Some(2));

    let replaced = leaf().or_else(|report| Err(report.with_message("context")));
    assert_eq!(replaced.unwrap_err().to_string(), "context");
}

#[test]
fn unwrap_family() {
    assert_eq!(ok(1).unwrap(), 1);
    assert_eq!(ok(1).expect("present"), 1);
    assert_eq!(leaf().unwrap_or(9), 9);
    assert_eq!(leaf().unwrap_or_else(|_| 8), 8);
    assert_eq!(leaf().unwrap_or_default(), 0);
    assert_eq!(leaf().unwrap_err().to_string(), "boom");
    assert_eq!(leaf().expect_err("an error").to_string(), "boom");
    // SAFETY: each value is the variant the call requires.
    unsafe {
        assert_eq!(ok(1).unwrap_unchecked(), 1);
        assert_eq!(leaf().unwrap_err_unchecked().to_string(), "boom");
    }
}

#[test]
#[should_panic(expected = "boom")]
fn unwrap_on_an_error_panics_with_the_report() {
    leaf().unwrap();
}

#[test]
fn borrowing_adapters_stay_tracked() {
    let mut value: Result<String> = Ok("ab".to_string());
    assert_eq!(value.as_ref().ok().map(String::len), Some(2));
    assert_eq!(value.as_deref().ok(), Some("ab"));

    // The views are `TrackedResult`s too, so the substituted `Ok` / `Err`
    // patterns apply to them.
    match value.as_mut() {
        Ok(s) => s.push('c'),
        Err(report) => panic!("unexpected {report}"),
    }
    if let Ok(s) = value.as_deref_mut() {
        s.make_ascii_uppercase();
    }
    assert_eq!(value.as_ref().map(|s| s.len()).unwrap(), 3);
    assert_eq!(value.ok().as_deref(), Some("ABC"));

    let failed = leaf();
    match failed.as_ref() {
        Ok(n) => panic!("unexpected {n}"),
        Err(report) => assert_eq!(report.to_string(), "boom"),
    }
    assert_eq!(failed.as_ref().copied().ok(), None);
}

#[test]
fn iteration() {
    let mut value: Result<u32> = Ok(1);
    assert_eq!(value.iter().copied().collect::<Vec<_>>(), [1]);

    for n in &mut value {
        *n += 1;
    }
    for n in value.iter_mut() {
        *n += 1;
    }
    let mut total = 0;
    for n in &value {
        total += *n;
    }
    assert_eq!(total, 3);

    assert_eq!(value.into_iter().collect::<Vec<_>>(), [3]);
    assert_eq!(leaf().into_iter().count(), 0);
}

#[test]
fn copied_cloned_transpose_flatten() {
    let n = 5;
    let by_ref: Result<&u32> = Ok(&n);
    assert_eq!(by_ref.copied().ok(), Some(5));
    let by_ref: Result<&u32> = Ok(&n);
    assert_eq!(by_ref.cloned().ok(), Some(5));

    let mut m = 6;
    let by_mut: Result<&mut u32> = Ok(&mut m);
    assert_eq!(by_mut.copied().ok(), Some(6));
    let by_mut: Result<&mut u32> = Ok(&mut m);
    assert_eq!(by_mut.cloned().ok(), Some(6));

    let some: Result<Option<u32>> = Ok(Some(1));
    assert_eq!(some.transpose().and_then(Result::ok), Some(1));
    let none: Result<Option<u32>> = Ok(None);
    assert!(none.transpose().is_none());

    let nested: Result<Result<u32>> = Ok(Ok(1));
    assert_eq!(nested.flatten().ok(), Some(1));
    let nested: Result<Result<u32>> = Ok(leaf());
    assert!(nested.flatten().is_err());
}

#[test]
fn sum_and_product_stop_at_the_first_error() {
    let total: Result<u32> = [Ok(1), Ok(2), Ok(3)].into_iter().sum();
    assert_eq!(total.ok(), Some(6));

    let product: Result<u32> = [Ok(2), Ok(3)].into_iter().product();
    assert_eq!(product.ok(), Some(6));

    let failed: Result<u32> = [Ok(1), leaf()].into_iter().sum();
    assert!(failed.is_err());
}

#[test]
fn usable_as_a_test_return_type() -> Result<()> {
    Ok(())
}
