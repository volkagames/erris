use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::Arc;
#[cfg(feature = "spantrace")]
use tracing::{dispatcher, span};

#[must_use]
pub struct Report {
    pub(crate) boxed: Box<ReportError>,
}

pub struct ReportError {
    pub(crate) inner: ReportType,
    pub(crate) track: ReportTrack,
}

#[derive(Debug)]
pub struct ReportTrack {
    pub(crate) location: &'static std::panic::Location<'static>,
    #[cfg(feature = "spantrace")]
    pub(crate) spantrace: crate::SpanTrace,
    /// Returns the span ID of the span that was active when this error was
    /// created.
    #[cfg(feature = "spantrace")]
    pub(crate) span_id: Option<span::Id>,
    #[cfg(feature = "backtrace")]
    pub(crate) backtrace: Option<crate::Backtrace>,
}

#[derive(Debug)]
pub enum ReportType {
    Transparent,
    Message(Cow<'static, str>),
    Report(Report),
    ArcReport(Arc<Report>),
    Error(Box<dyn std::error::Error + Send + Sync + 'static>),
    Wrapper(Report, Report),
}

impl ReportType {
    #[cfg(feature = "spantrace")]
    pub fn spantrace(&self) -> Option<&crate::SpanTrace> {
        match self {
            ReportType::Transparent => None,
            ReportType::Message(_) => None,
            ReportType::Report(inner) => Some(inner.spantrace()),
            ReportType::ArcReport(inner) => Some(inner.spantrace()),
            // Foreign errors are already erased to `dyn Error`; on stable there
            // is no way to request their SpanTrace. The wrapping link captured
            // its own SpanTrace in `ReportTrack`, so nothing is lost for erris
            // reports — only a SpanTrace carried inside a third-party error is
            // unreachable here.
            ReportType::Error(_inner) => None,
            ReportType::Wrapper(cause, _message) => Some(cause.spantrace()),
        }
    }

    #[cfg(feature = "backtrace")]
    pub fn backtrace(&self) -> Option<&crate::Backtrace> {
        match self {
            ReportType::Transparent => None,
            ReportType::Message(_) => None,
            ReportType::Report(inner) => inner.backtrace(),
            ReportType::ArcReport(inner) => inner.backtrace(),
            // See the SpanTrace arm above: a foreign error's own backtrace is
            // unreachable on stable. The wrapping link's backtrace (in
            // `ReportTrack`) still covers erris reports.
            ReportType::Error(_inner) => None,
            ReportType::Wrapper(cause, message) => {
                if let Some(backtrace) = cause.backtrace() {
                    return Some(backtrace);
                }
                if let Some(backtrace) = message.backtrace() {
                    return Some(backtrace);
                }
                None
            }
        }
    }
}

#[macro_export]
macro_rules! new_track {
    () => {{
        ReportTrack {
            location: std::panic::Location::caller(),

            #[cfg(feature = "spantrace")]
            spantrace: $crate::SpanTrace::capture(),

            #[cfg(feature = "spantrace")]
            span_id: dispatcher::get_default(|dispatcher| dispatcher.current_span().id().cloned()),

            #[cfg(feature = "backtrace")]
            backtrace: Some($crate::Backtrace::force_capture()),
        }
    }};
    ($inner:expr) => {{
        ReportTrack {
            location: std::panic::Location::caller(),

            #[cfg(feature = "spantrace")]
            spantrace: match $inner.spantrace() {
                Some(v) => v.clone(),
                None => $crate::SpanTrace::capture(),
            },
            #[cfg(feature = "spantrace")]
            span_id: dispatcher::get_default(|dispatcher| dispatcher.current_span().id().cloned()),

            #[cfg(feature = "backtrace")]
            backtrace: match $inner.backtrace().is_some() {
                true => None,
                false => Some($crate::Backtrace::force_capture()),
            },
        }
    }};
}

#[macro_export]
macro_rules! new_report {
    ($inner:expr) => {{
        let inner = $inner;
        let track = new_track!(&inner);
        Report {
            boxed: Box::new(ReportError { inner, track }),
        }
    }};
}

/// Fast path for `E == ReportError`: rewrap the existing `ReportError` without
/// building a new one. Returns the value back in `Err` when `E` is a different
/// type, so the caller can wrap it normally.
#[track_caller]
pub(crate) fn reuse_report_error<E>(error: E) -> std::result::Result<Report, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    if std::any::TypeId::of::<E>() == std::any::TypeId::of::<ReportError>() {
        // SAFETY: TypeId check guarantees E is ReportError — identical layout.
        // ptr::read reinterprets the bits without touching padding; forget avoids
        // double-drop.
        let inner = unsafe { std::ptr::read(&error as *const E as *const ReportError) };
        std::mem::forget(error);
        Ok(Report {
            boxed: Box::new(inner),
        })
    } else {
        Err(error)
    }
}

impl Report {
    #[track_caller]
    pub fn new_transparent() -> Self {
        new_report!(ReportType::Transparent)
    }

    #[track_caller]
    pub fn from_error<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        match reuse_report_error(error) {
            Ok(report) => report,
            Err(error) => new_report!(ReportType::Error(Box::new(error))),
        }
    }

    #[track_caller]
    pub fn from_boxed<E>(error: Box<E>) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        new_report!(ReportType::Error(error))
    }

    #[track_caller]
    pub fn from_dyn_boxed(error: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        new_report!(ReportType::Error(error))
    }

    #[track_caller]
    pub fn from_report(error: Report) -> Self {
        new_report!(ReportType::Report(error))
    }

    #[track_caller]
    pub fn from_arc_report(error: Arc<Report>) -> Self {
        new_report!(ReportType::ArcReport(error))
    }

    #[track_caller]
    pub fn from_message<T: Into<Cow<'static, str>>>(message: T) -> Self {
        new_report!(ReportType::Message(message.into()))
    }

    #[track_caller]
    pub fn from_format_args(args: std::fmt::Arguments<'_>) -> Self {
        if let Some(message) = args.as_str() {
            Report::from_message(message)
        } else {
            Report::from_message(std::fmt::format(args))
        }
    }

    #[track_caller]
    pub fn from_context(cause: Report, message: Report) -> Self {
        new_report!(ReportType::Wrapper(cause, message))
    }

    /// Wrap `self` as the cause under a new error built from `err`.
    #[track_caller]
    pub fn with_err<E: crate::IntoReport>(self, err: E) -> Self {
        Self::from_context(self, err.into_report())
    }

    /// Wrap `self` as the cause under a boxed `dyn Error`.
    #[track_caller]
    pub fn with_dyn_boxed(self, err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::from_context(self, Self::from_dyn_boxed(err))
    }

    /// Wrap `self` as the cause under a new message. This message becomes the
    /// report's `Display`; `self` becomes the first entry under `Caused by:`.
    #[track_caller]
    pub fn with_message(self, message: impl Into<Cow<'static, str>>) -> Self {
        Self::from_context(self, Report::from_message(message.into()))
    }

    /// Wrap `self` as the cause under another report.
    #[track_caller]
    pub fn with_report(self, report: impl Into<Report>) -> Self {
        Self::from_context(self, report.into())
    }

    /// Add a location frame without changing the message. Use on a `Result` via
    /// [`TrackReport::track`](crate::TrackReport::track) instead when possible.
    #[track_caller]
    pub fn track(self) -> Self {
        Self::from_report(self)
    }

    /// Iterate over this report and every error beneath it, root first.
    pub fn chain(&self) -> crate::ReportIterator<'_> {
        crate::ReportIterator::new(crate::ReportLink::Own(&*self.boxed))
    }
}

impl ReportError {
    pub fn new(inner: ReportType) -> Self {
        ReportError {
            inner,
            track: new_track!(),
        }
    }

    pub fn location(&self) -> &'static std::panic::Location<'static> {
        self.track.location
    }

    #[cfg(feature = "spantrace")]
    pub fn spantrace(&self) -> &crate::SpanTrace {
        &self.track.spantrace
    }

    #[cfg(feature = "spantrace")]
    pub fn span_id(&self) -> Option<span::Id> {
        self.track.span_id.clone()
    }

    #[cfg(feature = "backtrace")]
    pub fn backtrace(&self) -> Option<&crate::Backtrace> {
        if let Some(backtrace) = &self.track.backtrace {
            return Some(backtrace);
        }

        self.inner.backtrace()
    }

    #[cfg(feature = "backtrace")]
    pub fn inner_backtrace(&self) -> Option<&crate::Backtrace> {
        self.track.backtrace.as_ref()
    }

    /// Whether this link has no message of its own: its `Display` forwards to
    /// the error it wraps, or is empty for [`new_transparent`](Report::new_transparent).
    /// Only a message report (`report!("...")`) is not transparent. The same
    /// test as [`ReportLink::is_transparent`](crate::ReportLink::is_transparent).
    pub fn is_transparent(&self) -> bool {
        !matches!(self.inner, ReportType::Message(_))
    }

    pub fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            ReportType::Error(inner) => Some(inner.as_ref()),
            ReportType::Report(inner) => Some(inner.as_ref()),
            ReportType::ArcReport(inner) => Some(&inner.boxed),
            ReportType::Wrapper(cause, _) => Some(&cause.boxed),
            ReportType::Transparent | ReportType::Message(_) => None,
        }
    }

    /// Find the first error of type `T` in the chain, borrowing it.
    pub fn unwrap_ref<T: std::error::Error + 'static>(&self) -> Option<&T> {
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<Self>() {
            // The use of unsafe here is necessary and idiomatic for this pattern.
            // SAFETY: We just checked that T is ReportError
            return Some(unsafe { &*(self as *const Self as *const T) });
        }
        match &self.inner {
            ReportType::Error(inner) => downcast_source_chain::<T>(inner.as_ref()),
            ReportType::Report(inner) => inner.unwrap_ref::<T>(),
            ReportType::ArcReport(inner) => inner.unwrap_ref::<T>(),
            ReportType::Wrapper(cause, message) => cause
                .unwrap_ref::<T>()
                .or_else(|| message.unwrap_ref::<T>()),
            ReportType::Transparent | ReportType::Message(_) => None,
        }
    }

    /// Mutable counterpart to [`unwrap_ref`](Self::unwrap_ref).
    ///
    /// Note: an `ArcReport` link cannot be traversed here — a shared `Arc` grants
    /// no unique mutable access — so a `T` reachable only behind an `Arc<Report>`
    /// is found by `unwrap_ref` but returns `None` from `unwrap_mut`.
    pub fn unwrap_mut<T: std::error::Error + 'static>(&mut self) -> Option<&mut T> {
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<Self>() {
            // SAFETY: We just checked that T is ReportError
            return Some(unsafe { &mut *(self as *mut Self as *mut T) });
        }
        match &mut self.inner {
            ReportType::Error(inner) => inner.downcast_mut::<T>(),
            ReportType::Report(inner) => inner.unwrap_mut::<T>(),
            ReportType::Wrapper(cause, message) => cause
                .unwrap_mut::<T>()
                .or_else(|| message.unwrap_mut::<T>()),
            ReportType::Transparent | ReportType::Message(_) | ReportType::ArcReport(_) => None,
        }
    }

    /// Like [`unwrap_ref`](Self::unwrap_ref): find the first `T` anywhere in the
    /// chain, descending nested reports, wrappers, and boxed `source()` chains.
    pub fn unwrap_recursive<T: std::error::Error + 'static>(&self) -> Option<&T> {
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<Self>() {
            // SAFETY: We just checked that T is ReportError
            return Some(unsafe { &*(self as *const Self as *const T) });
        }
        match &self.inner {
            ReportType::Transparent => None,
            ReportType::Message(_) => None,
            ReportType::Report(inner) => inner.unwrap_recursive::<T>(),
            ReportType::ArcReport(inner) => inner.unwrap_recursive::<T>(),
            ReportType::Error(inner) => downcast_source_chain::<T>(inner.as_ref()),
            ReportType::Wrapper(cause, message) => cause
                .unwrap_recursive::<T>()
                .or_else(|| message.unwrap_recursive::<T>()),
        }
    }
}

/// Walk a std error's `source()` chain, returning the first link that is a `T`.
///
/// A boxed `dyn Error` is opaque to `downcast_ref` beyond its own type, but the
/// Debug/JSON chain follows `source()`, so a `T` reachable there must also be
/// reachable here — otherwise a type printed in the chain could not be
/// recovered by `unwrap_ref` / `unwrap_recursive`.
fn downcast_source_chain<'a, T: std::error::Error + 'static>(
    error: &'a (dyn std::error::Error + 'static),
) -> Option<&'a T> {
    let mut current = Some(error);
    while let Some(err) = current {
        if let Some(found) = err.downcast_ref::<T>() {
            return Some(found);
        }
        current = err.source();
    }
    None
}

impl std::error::Error for ReportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source()
    }
}

impl erris_meta::ReportMeta for ReportError {
    fn report_location(&self) -> Option<&'static std::panic::Location<'static>> {
        Some(self.location())
    }

    fn report_is_transparent(&self) -> bool {
        self.is_transparent()
    }

    fn report_branch(
        &self,
    ) -> (
        Option<&dyn erris_meta::ReportMeta>,
        Option<&dyn erris_meta::ReportMeta>,
    ) {
        match &self.inner {
            ReportType::Transparent | ReportType::Message(_) => (None, None),
            ReportType::Report(inner) => (Some(&**inner), None),
            ReportType::ArcReport(inner) => (Some(&***inner), None),
            // A foreign error has no ReportMeta view; it is traversed via
            // std source(), not this method.
            ReportType::Error(_) => (None, None),
            ReportType::Wrapper(cause, message) => (Some(&**cause), Some(&**message)),
        }
    }

    #[cfg(feature = "spantrace")]
    fn report_spantrace(&self) -> Option<&crate::SpanTrace> {
        Some(self.spantrace())
    }
}

impl std::ops::Deref for Report {
    type Target = ReportError;

    fn deref(&self) -> &Self::Target {
        &self.boxed
    }
}

impl std::ops::DerefMut for Report {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.boxed
    }
}

impl<T: std::error::Error + Send + Sync + 'static> From<T> for Report {
    #[track_caller]
    fn from(error: T) -> Report {
        Report::from_error(error)
    }
}

impl From<Report> for Box<dyn std::error::Error + Send + Sync + 'static> {
    fn from(error: Report) -> Self {
        error.boxed
    }
}

impl From<Report> for Box<dyn std::error::Error + 'static> {
    fn from(error: Report) -> Self {
        Box::<dyn std::error::Error + Send + Sync>::from(error)
    }
}

impl AsRef<dyn std::error::Error + Send + Sync> for Report {
    fn as_ref(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &**self
    }
}

impl AsRef<dyn std::error::Error> for Report {
    fn as_ref(&self) -> &(dyn std::error::Error + 'static) {
        &**self
    }
}
