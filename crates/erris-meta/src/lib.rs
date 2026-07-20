//! Version-stable shared trait for `erris`.
//!
//! The only purpose of this crate is to provide a single trait whose `TypeId`
//! is identical across every `erris` build in a dependency graph. When two
//! different `erris` versions coexist in one binary, each has its own
//! `ReportError` type with a distinct `TypeId`, so `downcast_ref::<ReportError>`
//! fails across the version boundary. Routing metadata access through this
//! shared trait instead lets a newer `erris` read an older one's report and
//! vice versa.
//!
//! # Semver contract
//!
//! This crate MUST stay on `1.x` forever. Changing the [`ReportMeta`] signature
//! (or bumping to `2.0`) would split the `TypeId` again and defeat the entire
//! purpose. Add capability only via new default methods, never by altering
//! existing ones.

use std::error::Error;
use std::panic::Location;

/// Metadata that a `erris` report exposes to any other `erris` version.
///
/// `Error` is a supertrait so a `&dyn ReportMeta` can be upcast to
/// `&dyn Error` on stable (stabilized in Rust 1.86), giving `Display` and
/// `source()` without a separate stored pointer.
pub trait ReportMeta: Error + Send + Sync + 'static {
    /// Location captured when this report link was created.
    fn report_location(&self) -> Option<&'static Location<'static>>;

    /// Whether this link is a transparent (message-less) wrapper.
    fn report_is_transparent(&self) -> bool;

    /// The child links of this report, as `dyn ReportMeta`.
    ///
    /// Returned through the shared trait (not `dyn Error::source`) so a report
    /// built by a different `erris` version can still be traversed with both
    /// branches intact — e.g. a context wrapper's `(cause, message)` pair.
    /// `(None, None)` for a leaf.
    fn report_branch(&self) -> (Option<&dyn ReportMeta>, Option<&dyn ReportMeta>);

    /// SpanTrace captured for this report link, if any.
    #[cfg(feature = "spantrace")]
    fn report_spantrace(&self) -> Option<&tracing_error::SpanTrace>;
}
