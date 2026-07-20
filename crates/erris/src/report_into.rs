use crate::Report;

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
