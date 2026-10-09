//! Ergonomic extension traits on `Result` and `Option` for producing a
//! [`Report`]. Each converts the error/`None` case into a `Report` while
//! preserving the caller location via `#[track_caller]`.
//!
//! They are implemented for std `Result` and hand back [`crate::Result`], so with
//! the `tracked` feature their outcome is the tracking result.

use crate::{IntoReport, Report};
use std::borrow::Cow;
use std::fmt::Arguments;

/// A message for a report built on the error path: a `&'static str`, a
/// `String`, a `Cow<'static, str>`, or [`format_args!`].
///
/// `format_args!` only borrows its arguments, so the message is formatted
/// when the report is built, not on the happy path. A literal without
/// arguments is not formatted at all.
///
/// ```
/// use erris::OkOrReport;
///
/// let code = "achieve";
/// let templates: Vec<&str> = Vec::new();
/// let err = templates
///     .iter()
///     .find(|t| **t == code)
///     .ok_or_report(format_args!("missing template `{code}`"))
///     .unwrap_err();
/// assert_eq!(err.to_string(), "missing template `achieve`");
/// ```
pub trait ReportMessage {
    #[track_caller]
    fn into_message_report(self) -> Report;
}

impl ReportMessage for &'static str {
    #[track_caller]
    fn into_message_report(self) -> Report {
        Report::from_message(self)
    }
}

impl ReportMessage for String {
    #[track_caller]
    fn into_message_report(self) -> Report {
        Report::from_message(self)
    }
}

impl ReportMessage for Cow<'static, str> {
    #[track_caller]
    fn into_message_report(self) -> Report {
        Report::from_message(self)
    }
}

impl ReportMessage for Arguments<'_> {
    #[track_caller]
    fn into_message_report(self) -> Report {
        Report::from_format_args(self)
    }
}

/// Convert an `Option`/`Result` into a [`crate::Result`], supplying a message
/// (or a fresh report) for the empty/error case.
pub trait OkOrReport<T> {
    #[track_caller]
    fn ok_or_report<M>(self, message: M) -> crate::Result<T>
    where
        M: ReportMessage;

    #[track_caller]
    fn ok_or_report_with<D, F>(self, f: F) -> crate::Result<T>
    where
        D: IntoReport,
        F: FnOnce() -> D;
}

impl<T> OkOrReport<T> for Option<T> {
    #[track_caller]
    fn ok_or_report<M>(self, message: M) -> crate::Result<T>
    where
        M: ReportMessage,
    {
        match self {
            Some(v) => crate::Result::Ok(v),
            None => crate::Result::Err(message.into_message_report()),
        }
    }

    #[track_caller]
    fn ok_or_report_with<D, F>(self, f: F) -> crate::Result<T>
    where
        D: IntoReport,
        F: FnOnce() -> D,
    {
        match self {
            Some(v) => crate::Result::Ok(v),
            None => crate::Result::Err(f().into_report()),
        }
    }
}

impl<T, E: std::error::Error + Send + Sync + 'static> OkOrReport<T> for std::result::Result<T, E> {
    #[track_caller]
    fn ok_or_report<M>(self, message: M) -> crate::Result<T>
    where
        M: ReportMessage,
    {
        match self {
            Ok(v) => crate::Result::Ok(v),
            Err(err) => crate::Result::Err(
                Report::from_error(err).with_report(message.into_message_report()),
            ),
        }
    }

    #[track_caller]
    fn ok_or_report_with<D, F>(self, f: F) -> crate::Result<T>
    where
        D: IntoReport,
        F: FnOnce() -> D,
    {
        match self {
            Ok(v) => crate::Result::Ok(v),
            Err(err) => crate::Result::Err(err.into_report().with_err(f())),
        }
    }
}

/// Turn any convertible error into a tracked `Report`, adding a location frame.
pub trait TrackReport<T> {
    #[track_caller]
    fn track(self) -> crate::Result<T>;
}

impl<T, E: IntoReport> TrackReport<T> for Result<T, E> {
    #[track_caller]
    fn track(self) -> crate::Result<T> {
        match self {
            Ok(t) => crate::Result::Ok(t),
            Err(e) => crate::Result::Err(e.into_report().track()),
        }
    }
}

/// Wrap a `Result`'s error with a message (or a lazily built report).
pub trait WrapReport<T, E> {
    #[track_caller]
    fn wrap_report<M>(self, err: M) -> crate::Result<T>
    where
        M: ReportMessage;

    #[track_caller]
    fn wrap_report_with<D, F>(self, f: F) -> crate::Result<T>
    where
        D: IntoReport,
        F: FnOnce() -> D;
}

impl<T, E> WrapReport<T, E> for Result<T, E>
where
    E: IntoReport,
{
    #[track_caller]
    fn wrap_report<M>(self, err: M) -> crate::Result<T>
    where
        M: ReportMessage,
    {
        match self {
            Ok(t) => crate::Result::Ok(t),
            Err(e) => crate::Result::Err(e.into_report().with_report(err.into_message_report())),
        }
    }

    #[track_caller]
    fn wrap_report_with<D, F>(self, f: F) -> crate::Result<T>
    where
        D: IntoReport,
        F: FnOnce() -> D,
    {
        match self {
            Ok(t) => crate::Result::Ok(t),
            Err(e) => crate::Result::Err(e.into_report().with_err(f())),
        }
    }
}

/// Like [`WrapReport`], but for a `Result` whose error is a boxed `dyn Error`.
pub trait WrapBoxReport<T> {
    #[track_caller]
    fn wrap_report<M>(self, err: M) -> crate::Result<T>
    where
        M: ReportMessage;

    #[track_caller]
    fn wrap_report_with<D, F>(self, f: F) -> crate::Result<T>
    where
        D: IntoReport,
        F: FnOnce() -> D;
}

impl<T> WrapBoxReport<T> for Result<T, Box<dyn std::error::Error + Send + Sync + 'static>> {
    #[track_caller]
    fn wrap_report<M>(self, err: M) -> crate::Result<T>
    where
        M: ReportMessage,
    {
        match self {
            Ok(t) => crate::Result::Ok(t),
            Err(e) => {
                crate::Result::Err(Report::from_dyn_boxed(e).with_report(err.into_message_report()))
            }
        }
    }

    #[track_caller]
    fn wrap_report_with<D, F>(self, f: F) -> crate::Result<T>
    where
        D: IntoReport,
        F: FnOnce() -> D,
    {
        match self {
            Ok(t) => crate::Result::Ok(t),
            Err(e) => crate::Result::Err(Report::from_dyn_boxed(e).with_err(f())),
        }
    }
}

/// `into_std()` on a std `Result<T, Report>` (or `&Report` / `&mut Report`, as
/// `as_ref` / `as_mut` give): the identity. With `tracked` on,
/// `TrackedResult::into_std` is the inherent method of the same name, so
/// handing a result to an API bounded on std `Result` — `try_join_all`, a
/// diesel `transaction`, `OnceCell::get_or_try_init` — reads the same in both
/// modes. Exported from [`prelude`](crate::prelude).
pub trait IntoStd<T, E> {
    fn into_std(self) -> Result<T, E>;
}

impl<T> IntoStd<T, Report> for Result<T, Report> {
    fn into_std(self) -> Self {
        self
    }
}

impl<'a, T> IntoStd<T, &'a Report> for Result<T, &'a Report> {
    fn into_std(self) -> Self {
        self
    }
}

impl<'a, T> IntoStd<T, &'a mut Report> for Result<T, &'a mut Report> {
    fn into_std(self) -> Self {
        self
    }
}
