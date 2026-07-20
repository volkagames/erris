use crate::prelude::*;
use crate::{ReportError, ReportIterator, ReportType};
use std::fmt::{Debug, Display, Write as _};

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.boxed, f)
    }
}

impl Debug for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.boxed, f)
    }
}

impl Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.inner {
            ReportType::Transparent => Ok(()),
            ReportType::Message(message) => f.write_str(message),
            ReportType::Report(inner) => Display::fmt(inner, f),
            ReportType::ArcReport(inner) => Display::fmt(inner, f),
            ReportType::Error(inner) => Display::fmt(inner, f),
            ReportType::Wrapper(_cause, message) => Display::fmt(message, f),
        }
    }
}

impl Debug for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if f.alternate() {
            let mut dbg = f.debug_struct("ReportError");

            dbg.field("inner", &self.inner);
            dbg.field("location", self.location());

            #[cfg(feature = "spantrace")]
            dbg.field("spantrace", self.spantrace());

            #[cfg(feature = "backtrace")]
            if let Some(backtrace) = self.backtrace() {
                dbg.field("backtrace", backtrace);
            }

            return dbg.finish();
        }

        debug_error_chain(self, f)
    }
}

pub fn debug_error_chain(error: &ReportError, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{error}")?;

    let causes = collect_causes_without_consecutive_duplicates(error);
    if !causes.is_empty() {
        write!(f, "\n\nCaused by:")?;
        for (n, cause) in causes.iter().enumerate() {
            writeln!(f)?;
            write!(indenter::indented(f).ind(n), "{cause}")?;
        }
    }

    write!(f, "\n\nLocation:")?;
    let chain = collect_locations_without_consecutive_duplicates(error);
    for (n, location) in chain.iter().enumerate() {
        writeln!(f)?;
        write!(indenter::indented(f).ind(n), "{location}")?;
    }

    #[cfg(feature = "spantrace")]
    {
        let spantrace = error.spantrace();
        write!(f, "\n\nSpanTrace: \n{spantrace}")?;
    }

    #[cfg(feature = "backtrace")]
    if let Some(backtrace) = error.backtrace() {
        write!(f, "\n\nBacktrace: \n{backtrace}")?;
    }

    Ok(())
}

/// The cause chain below `error`, as rendered strings.
///
/// `error`'s own Display is the top-level rendering (the Debug header / JSON
/// `message`), so the root is skipped. A context wrapper and its message branch
/// render identically, so consecutive duplicate renderings are collapsed — the
/// same rule [`collect_locations_without_consecutive_duplicates`] applies to
/// locations.
pub(crate) fn collect_causes_without_consecutive_duplicates(error: &ReportError) -> Vec<String> {
    let mut causes = Vec::new();
    let mut last = error.to_string();
    for link in ReportIterator::new(crate::ReportLink::Own(error))
        .skip(1)
        .filter(|link| !link.is_transparent())
    {
        let rendered = link.as_error().to_string();
        if rendered != last {
            causes.push(rendered.clone());
            last = rendered;
        }
    }
    causes
}

pub(crate) fn collect_locations_without_consecutive_duplicates(
    error: &ReportError,
) -> Vec<&'static std::panic::Location<'static>> {
    let mut last_location: Option<&std::panic::Location> = None;
    let mut result = Vec::new();

    for link in ReportIterator::new(crate::ReportLink::Own(error)) {
        if let Some(location) = link.location() {
            // Check if the location is different from the last one stored
            if last_location.is_none_or(|last| !is_same_location(last, location)) {
                result.push(location);
            }
            last_location = Some(location);
        }
    }

    result
}

#[cfg(feature = "keep-duplicate-location")]
fn is_same_location(
    lhs: &std::panic::Location<'static>,
    rhs: &'static std::panic::Location<'static>,
) -> bool {
    lhs == rhs
}

#[cfg(not(feature = "keep-duplicate-location"))]
fn is_same_location(
    lhs: &std::panic::Location<'static>,
    rhs: &'static std::panic::Location<'static>,
) -> bool {
    lhs.file() == rhs.file() && lhs.line() == rhs.line()
}

