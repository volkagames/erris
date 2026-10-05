//! `TrackedResult` over an error type of its own: one that implements
//! `TrackedError`, the way an HTTP layer's typed error does.
#![cfg(feature = "tracked")]

use erris::prelude::*;
use erris::report;
use erris::tracked::{Result, TrackedError};
use std::panic::Location;
use std::result::Result as StdResult;

/// Records every hop, starting with the line it was raised on.
#[derive(Debug, PartialEq)]
struct ApiError {
    code: &'static str,
    lines: Vec<u32>,
}

impl ApiError {
    #[track_caller]
    fn new(code: &'static str) -> Self {
        ApiError {
            code,
            lines: vec![Location::caller().line()],
        }
    }
}

impl TrackedError for ApiError {
    #[track_caller]
    fn track_hop(mut self) -> Self {
        self.lines.push(Location::caller().line());
        self
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code)
    }
}

impl std::error::Error for ApiError {}

impl From<std::num::ParseIntError> for ApiError {
    #[track_caller]
    fn from(_: std::num::ParseIntError) -> Self {
        ApiError::new("invalid_argument")
    }
}

const LEAF: u32 = line!() + 2;
fn leaf() -> Result<u32, ApiError> {
    Err(ApiError::new("not_found"))
}

#[test]
fn question_mark_tracks_an_error_of_its_own() {
    const MID: u32 = line!() + 2;
    fn mid() -> Result<u32, ApiError> {
        let n = leaf()?;
        Ok(n)
    }
    const TOP: u32 = line!() + 2;
    fn top() -> Result<u32, ApiError> {
        let n = mid()?;
        Ok(n)
    }

    let err = top().unwrap_err();
    assert_eq!(err.code, "not_found");
    assert_eq!(err.lines, [LEAF, MID, TOP]);
}

#[test]
fn std_error_is_converted_then_tracked() {
    const PARSE: u32 = line!() + 2;
    fn parse(s: &str) -> Result<u32, ApiError> {
        let n: u32 = s.parse()?;
        Ok(n)
    }

    assert_eq!(parse("7").ok(), Some(7));
    let err = parse("x").unwrap_err();
    assert_eq!(err.code, "invalid_argument");
    // Raised by `From` on the `?` line, then tracked by the same `?`; a
    // `TrackedError` of its own decides whether to collapse the two.
    assert_eq!(err.lines, [PARSE, PARSE]);
}

#[test]
fn std_result_with_the_same_error_is_tracked() {
    const STD_LEAF: u32 = line!() + 2;
    fn std_leaf() -> StdResult<u32, ApiError> {
        StdResult::Err(ApiError::new("not_found"))
    }
    const HOP: u32 = line!() + 2;
    fn hop() -> Result<u32, ApiError> {
        let n = std_leaf()?;
        Ok(n)
    }

    assert_eq!(hop().unwrap_err().lines, [STD_LEAF, HOP]);
}

#[test]
fn boundary_into_a_std_result_tracks() {
    const HANDLER: u32 = line!() + 2;
    fn handler() -> StdResult<u32, ApiError> {
        let n = leaf()?;
        StdResult::Ok(n)
    }

    assert_eq!(handler().unwrap_err().lines, [LEAF, HANDLER]);
}

#[test]
fn an_error_of_its_own_becomes_a_report() {
    // Library code under a handler calling back into handler code.
    const LIB: u32 = line!() + 2;
    fn lib() -> Result<u32> {
        let n = leaf()?;
        Ok(n)
    }

    let report = lib().unwrap_err();
    assert_eq!(report.to_string(), "not_found");
    assert_eq!(report.location().line(), LIB);
}

#[test]
fn std_methods_hand_out_the_error() {
    let mut res = leaf();
    assert_eq!(res.as_ref().err().map(|e| e.code), Some("not_found"));
    assert_eq!(res.as_mut().err().map(|e| e.code), Some("not_found"));
    assert_eq!(
        leaf()
            .or_else(|e| if e.code == "not_found" { Ok(0) } else { Err(e) })
            .ok(),
        Some(0)
    );
    assert_eq!(leaf().or(Ok(1)).ok(), Some(1));
    assert_eq!(leaf().map_err(|e| e.code), StdResult::Err("not_found"));
    let from_std: Result<u32, ApiError> = StdResult::Ok(2).into();
    assert_eq!(from_std.ok(), Some(2));
}

#[test]
fn iterators_collect_into_a_tracked_result() {
    let all: Result<Vec<u32>, ApiError> = [Ok(1), Ok(2)].into_iter().collect();
    assert_eq!(all.ok(), Some(vec![1, 2]));

    let failed: Result<Vec<u32>, ApiError> = [Ok(1), leaf()].into_iter().collect();
    assert_eq!(failed.unwrap_err().code, "not_found");

    let sum: Result<u32, ApiError> = [Ok(1), Ok(2)].into_iter().sum();
    assert_eq!(sum.ok(), Some(3));
}

#[test]
fn a_report_closure_still_infers_its_error() {
    // The error type of a closure or an async block is inferred from its `?`
    // on a `TrackedResult`, as before other error types could be named.
    fn report_leaf() -> Result<u32> {
        Err(report!("boom"))
    }
    let run = || {
        let n = report_leaf()?;
        Ok(n)
    };
    let report: Report = run().unwrap_err();
    assert_eq!(report.to_string(), "boom");
}

#[cfg(feature = "axum")]
mod axum {
    use super::*;
    use axum_core::response::{IntoResponse, Response};

    impl IntoResponse for ApiError {
        fn into_response(self) -> Response {
            let mut response = self.code.into_response();
            *response.status_mut() = http::StatusCode::NOT_FOUND;
            response
        }
    }

    #[test]
    fn a_tracked_result_is_a_response() {
        fn handler(found: bool) -> Result<&'static str, ApiError> {
            if found {
                Ok("user")
            } else {
                leaf().map(|_| "unreachable")
            }
        }

        assert_eq!(handler(true).into_response().status(), http::StatusCode::OK);
        assert_eq!(
            handler(false).into_response().status(),
            http::StatusCode::NOT_FOUND
        );
    }
}
