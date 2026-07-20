//! Ergonomic extension traits on `Result` and `Option` for producing a
//! [`Report`]. Each converts the error/`None` case into a `Report` while
//! preserving the caller location via `#[track_caller]`.

use crate::{IntoReport, Report};
use std::borrow::Cow;

/// Convert an `Option`/`Result` into a `Result<T, Report>`, supplying a message
/// (or a fresh report) for the empty/error case.
pub trait OkOrReport<T> {
    #[track_caller]
    fn ok_or_report<M>(self, message: M) -> Result<T, Report>
    where
        M: Into<Cow<'static, str>>;

    #[track_caller]
    fn ok_or_report_with<D, F>(self, f: F) -> Result<T, Report>
    where
        D: IntoReport,
        F: FnOnce() -> D;
}

impl<T> OkOrReport<T> for Option<T> {
    #[track_caller]
    fn ok_or_report<M>(self, message: M) -> Result<T, Report>
    where
        M: Into<Cow<'static, str>>,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(Report::from_message(message.into())),
        }
    }

    #[track_caller]
    fn ok_or_report_with<D, F>(self, f: F) -> Result<T, Report>
    where
        D: IntoReport,
        F: FnOnce() -> D,
    {
        match self {
            Some(v) => Ok(v),
            None => Err(f().into_report()),
        }
    }
}

impl<T, E: std::error::Error + Send + Sync + 'static> OkOrReport<T> for std::result::Result<T, E> {
    #[track_caller]
    fn ok_or_report<M>(self, message: M) -> Result<T, Report>
    where
        M: Into<Cow<'static, str>>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(Report::from_error(err).with_message(message)),
        }
    }

    #[track_caller]
    fn ok_or_report_with<D, F>(self, f: F) -> Result<T, Report>
    where
        D: IntoReport,
        F: FnOnce() -> D,
    {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(err.into_report().with_err(f())),
        }
    }
}

/// Turn any convertible error into a tracked `Report`, adding a location frame.
pub trait TrackReport<T> {
    #[track_caller]
    fn track(self) -> Result<T, Report>;
}

impl<T, E: IntoReport> TrackReport<T> for Result<T, E> {
    #[track_caller]
    fn track(self) -> Result<T, Report> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.into_report().track()),
        }
    }
}

/// Wrap a `Result`'s error with a message (or a lazily built report).
pub trait WrapReport<T, E> {
    #[track_caller]
    fn wrap_report<M>(self, err: M) -> Result<T, Report>
    where
        M: Into<Cow<'static, str>>;

    #[track_caller]
    fn wrap_report_with<D, F>(self, f: F) -> Result<T, Report>
    where
        D: IntoReport,
        F: FnOnce() -> D;
}

impl<T, E> WrapReport<T, E> for Result<T, E>
where
    E: IntoReport,
{
    #[track_caller]
    fn wrap_report<M>(self, err: M) -> Result<T, Report>
    where
        M: Into<Cow<'static, str>>,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.into_report().with_err(Report::from_message(err.into()))),
        }
    }

    #[track_caller]
    fn wrap_report_with<D, F>(self, f: F) -> Result<T, Report>
    where
        D: IntoReport,
        F: FnOnce() -> D,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.into_report().with_err(f())),
        }
    }
}

/// Like [`WrapReport`], but for a `Result` whose error is a boxed `dyn Error`.
pub trait WrapBoxReport<T> {
    #[track_caller]
    fn wrap_report<M>(self, err: M) -> Result<T, Report>
    where
        M: Into<Cow<'static, str>>;

    #[track_caller]
    fn wrap_report_with<D, F>(self, f: F) -> Result<T, Report>
    where
        D: IntoReport,
        F: FnOnce() -> D;
}

impl<T> WrapBoxReport<T> for Result<T, Box<dyn std::error::Error + Send + Sync + 'static>> {
    #[track_caller]
    fn wrap_report<M>(self, err: M) -> Result<T, Report>
    where
        M: Into<Cow<'static, str>>,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(Report::from_dyn_boxed(e).with_err(Report::from_message(err.into()))),
        }
    }

    #[track_caller]
    fn wrap_report_with<D, F>(self, f: F) -> Result<T, Report>
    where
        D: IntoReport,
        F: FnOnce() -> D,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(Report::from_dyn_boxed(e).with_err(f())),
        }
    }
}
