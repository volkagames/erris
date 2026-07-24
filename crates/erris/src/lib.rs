//! `erris` is an error-reporting library in the spirit of `anyhow` / `eyre`:
//! a single [`Report`] type that any error converts into, capturing the caller
//! location (and, with features, a span trace and backtrace) at each step.
//!
//! # Which method do I call?
//!
//! The verb depends on what you are holding.
//!
//! | You have… | To build a report | To add context |
//! |-----------|-------------------|----------------|
//! | a message / error value | [`report!`] | — |
//! | a [`Report`] | — | [`Report::with_message`], [`Report::with_report`], [`Report::with_err`] |
//! | a `Result<T, E>` | — | [`WrapReport::wrap_report`], [`TrackReport::track`], [`OkOrReport::ok_or_report`] |
//! | an `Option<T>` | — | [`OkOrReport::ok_or_report`] |
//! | a `Box<dyn Error>` result | — | [`WrapBoxReport::wrap_report`] |
//!
//! `with_*` operates on a `Report` you already hold; `wrap_report` / `track` /
//! `ok_or_report` operate on the `Result`/`Option` in the `?` position. Both
//! record a fresh location via `#[track_caller]`.
//!
//! # Example
//!
//! ```
//! use erris::prelude::*;
//! use erris::report;
//!
//! fn load() -> erris::Result<()> {
//!     let value: Result<(), std::io::Error> = Err(std::io::Error::other("disk gone"));
//!     value.wrap_report("could not load config")?; // adds context + location
//!     Ok(())
//! }
//!
//! let err = load().unwrap_err();
//! assert_eq!(err.to_string(), "could not load config");
//! ```
//!
//! # Recovering a typed error
//!
//! [`unwrap_ref`](crate::ReportError::unwrap_ref) /
//! [`unwrap_recursive`](crate::ReportError::unwrap_recursive) (reached on a
//! `Report` through `Deref`) downcast back to a concrete error type anywhere in
//! the chain, including a boxed error's own `source()` chain.
//!
//! # Features
//!
//! - `spantrace` (default): capture a `tracing` span trace per report link.
//! - `to_json` (implies `spantrace`): [`Report::to_json`] and the
//!   `tracing_fields` span-field layer.
//! - `backtrace`: capture a `std::backtrace::Backtrace`.
//! - `serde_json_value`: inject arbitrary `serde_json::Value` into layer fields
//!   via [`JsonVisitor::record_json`](tracing_fields::JsonVisitor::record_json).
#![warn(
    bad_style,
    dead_code,
    improper_ctypes,
    missing_debug_implementations,
    no_mangle_generic_items,
    non_shorthand_field_patterns,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    unconditional_recursion,
    unreachable_pub,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    unused,
    while_true
)]

mod report;
mod report_debug;
mod report_into;
mod report_iterator;
#[doc(hidden)]
pub mod report_kind;
mod result_ext;

pub use report::*;
pub use report_debug::*;
pub use report_into::*;
pub use report_iterator::*;
pub use report_kind::*;
pub use result_ext::*;

#[cfg(feature = "to_json")]
mod report_to_json;

#[cfg(feature = "to_json")]
pub use report_to_json::*;

pub type Result<T, E = Report> = std::result::Result<T, E>;

// The tracing_fields layer stores span fields as `serde_json::Map`, so it needs
// serde_json — which only the `to_json` feature pulls in. It is consumed solely
// by the JSON output path, so gate it on `to_json` (which implies `spantrace`).
#[cfg(feature = "to_json")]
pub mod tracing_fields;

// re export
#[cfg(feature = "backtrace")]
pub use std::backtrace::Backtrace;
#[cfg(feature = "spantrace")]
pub use tracing_error::SpanTrace;

pub mod prelude {
    pub use crate::{BoxIntoReport, OkOrReport, Report, TrackReport, WrapBoxReport, WrapReport};
}

/// Build a [`Report`] from a message, a format string, an error value, or
/// another report.
///
/// - `report!()` — a transparent (message-less) report, useful as an anchor for
///   a later `with_message`.
/// - `report!("literal")` / `report!("x = {x}")` — a message report.
/// - `report!(value)` — dispatches on the value: a `Display`-able string, a
///   `std::error::Error`, a boxed `dyn Error`, an `Arc<Report>`, or a `Report`.
/// - `report!("{}", arg)` — a formatted message report.
///
/// The caller location is captured at the macro site.
///
/// ```
/// let e = erris::report!("bad value: {}", 42);
/// assert_eq!(e.to_string(), "bad value: 42");
/// ```
#[macro_export]
macro_rules! report {
    () => ({
        $crate::Report::new_transparent()
    });
    ($msg:literal $(,)?) => ({
        $crate::Report::from_format_args(std::format_args!($msg))
    });
    ($err:expr $(,)?) => ({
        use $crate::report_kind::*;
        let error = match $err {
            error => (&error).report_kind().new_report(error),
        };
        error
    });
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Report::from_message(format!($fmt, $($arg)*))
    };
}
