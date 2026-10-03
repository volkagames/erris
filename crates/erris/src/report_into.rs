use crate::Report;

/// A value that becomes a [`Report`]: any `std::error::Error + Send + Sync +
/// 'static`, or a `Report` itself. Anything else is rejected with a hint at the
/// way in:
///
/// ```compile_fail
/// struct NotAnError;
/// // error[E0277]: `NotAnError` cannot become an `erris::Report`
/// let _ = erris::report!("base").with_err(NotAnError);
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot become an `erris::Report`",
    label = "not a `std::error::Error + Send + Sync + 'static`, nor a `Report`",
    note = "implement `std::error::Error` for it, or build a report from it with \
            `erris::report!(..)` (any `Display` value works)",
    note = "a `Box<dyn Error + Send + Sync>` converts through `BoxIntoReport` / \
            `WrapBoxReport`"
)]
pub trait IntoReport {
    #[track_caller]
    fn into_report(self) -> Report
    where
        Self: Sized;
}

impl<E: std::error::Error + Send + Sync + 'static> IntoReport for E {
    #[track_caller]
    fn into_report(self) -> Report {
        match crate::reuse_report_error(self) {
            Ok(report) => report,
            Err(error) => Report::from_error(error),
        }
    }
}

impl IntoReport for Report {
    #[track_caller]
    fn into_report(self) -> Report {
        self
    }
}

pub trait BoxIntoReport {
    #[track_caller]
    fn into_report(self) -> Report
    where
        Self: Sized;
}

impl BoxIntoReport for Box<dyn std::error::Error + Send + Sync + 'static> {
    #[track_caller]
    fn into_report(self) -> Report {
        Report::from_dyn_boxed(self)
    }
}
