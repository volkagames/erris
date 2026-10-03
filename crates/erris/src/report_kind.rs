#![allow(missing_debug_implementations, missing_docs)]

use crate::Report;
use std::borrow::Cow;

pub struct FromReport;

pub trait FromReportKind: Sized {
    #[inline]
    fn report_kind(&self) -> FromReport {
        FromReport
    }
}

impl FromReportKind for Report {}

impl FromReport {
    #[track_caller]
    pub fn new_report(self, value: Report) -> Report {
        Report::from_report(value)
    }

    /// A report that becomes the cause of another one is already a chain:
    /// keep it as is instead of nesting it one level deeper.
    #[track_caller]
    pub fn into_cause(self, value: Report) -> Report {
        value
    }
}

pub struct FromStdError;

pub trait FromStdErrorKind: Sized {
    #[inline]
    fn report_kind(&self) -> FromStdError {
        FromStdError
    }
}

impl<T> FromStdErrorKind for T where T: std::error::Error + Send + Sync + 'static {}

impl FromStdError {
    #[track_caller]
    pub fn new_report<T: std::error::Error + Send + Sync + 'static>(self, value: T) -> Report {
        Report::from_error(value)
    }

    #[track_caller]
    pub fn into_cause<T: std::error::Error + Send + Sync + 'static>(self, value: T) -> Report {
        self.new_report(value)
    }
}

pub struct FromMessage;

pub trait FromMessageKind: Sized {
    #[inline]
    fn report_kind(&self) -> FromMessage {
        FromMessage
    }
}

impl<T> FromMessageKind for T where T: Into<Cow<'static, str>> {}

impl FromMessage {
    #[track_caller]
    pub fn new_report<T: Into<Cow<'static, str>>>(self, value: T) -> Report {
        Report::from_message(value.into())
    }

    #[track_caller]
    pub fn into_cause<T: Into<Cow<'static, str>>>(self, value: T) -> Report {
        self.new_report(value)
    }
}

pub struct FromDynBox;

pub trait FromDynBoxKind: Sized {
    #[inline]
    fn report_kind(&self) -> FromDynBox {
        FromDynBox
    }
}

impl FromDynBoxKind for Box<dyn std::error::Error + Send + Sync + 'static> {}

impl FromDynBox {
    #[track_caller]
    pub fn new_report(self, error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Report {
        Report::from_dyn_boxed(error)
    }

    #[track_caller]
    pub fn into_cause(self, error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Report {
        self.new_report(error)
    }
}

pub struct FromArc;

pub trait FromArcKind: Sized {
    #[inline]
    fn report_kind(&self) -> FromArc {
        FromArc
    }
}

impl FromArcKind for std::sync::Arc<Report> {}

impl FromArc {
    #[track_caller]
    pub fn new_report(self, error: std::sync::Arc<Report>) -> Report {
        Report::from_arc_report(error)
    }

    #[track_caller]
    pub fn into_cause(self, error: std::sync::Arc<Report>) -> Report {
        self.new_report(error)
    }
}
