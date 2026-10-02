//! [`TrackedResult`]: a result whose `?` adds a location frame on every hop,
//! with no `.track()` at the call site.
//!
//! Nightly only: the hook is a `#[track_caller]` [`FromResidual`] impl, which
//! needs `try_trait_v2`. With std `Result` the same hop goes through core's
//! reflexive `From<Report> for Report`, which cannot be intercepted.
//!
//! The type is [`TrackedResult`], also exported at the crate root. With this
//! feature on, [`crate::Result`] names it instead of the std alias. This module
//! exports it under the alias [`Result`] too, next to its variants: import all
//! three to substitute it for std `Result` in a module, so function bodies read
//! the same:
//!
//! ```
//! use erris::tracked::{Ok, Result};
//!
//! fn parse(s: &str) -> Result<u32> {
//!     let n: u32 = s.parse()?; // std error -> Report, located here
//!     Ok(n)
//! }
//!
//! fn double(s: &str) -> Result<u32> {
//!     let n = parse(s)?; // Report -> Report, a frame is added here
//!     Ok(n * 2)
//! }
//!
//! assert!(double("x").is_err());
//! ```
//!
//! The error type is fixed to [`Report`]. A function whose signature a foreign
//! trait dictates keeps returning std `Result`; `?` converts in both directions
//! and tracks in both.
//!
//! The rest of erris follows: `track` and `wrap_report` are methods of this type
//! too, and the extension traits on std `Result` / `Option` and the
//! [`be`](crate::be) macros return it. A std `Result<T, Report>` from elsewhere
//! is consumed with `?`, or with `.into()` in tail position.

use crate::{IntoReport, Report};
use std::borrow::Cow;
use std::convert::Infallible;
use std::iter::{Product, Sum};
use std::ops::{ControlFlow, FromResidual, Residual, Try};
use std::process::{ExitCode, Termination};
use std::result::Result as StdResult;

/// The error parameter keeps std's `Result<T, E>` shape, but it is not free: it
/// is a [`Report`], owned or — as [`as_ref`](TrackedResult::as_ref) and friends
/// produce — borrowed. Naming any other error type is rejected with an
/// explanation (see [`ReportOnly`]) rather than a bare arity error.
#[derive(Debug)]
#[must_use]
pub enum TrackedResult<T, E: ReportOnly = Report> {
    Ok(T),
    Err(E),
}

/// Marks the error types a [`TrackedResult`] accepts: [`Report`], `&Report` and
/// `&mut Report`.
///
/// ```compile_fail
/// fn load() -> erris::Result<(), std::io::Error> {
///     todo!()
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`erris::Result` takes no error type while the `tracked` feature is on",
    label = "the error is always an `erris::Report`, not `{Self}`",
    note = "write `erris::Result<T>`; for a result that is not tracked, spell out \
            `std::result::Result<T, {Self}>`"
)]
pub trait ReportOnly: sealed::Sealed {}

impl ReportOnly for Report {}
impl ReportOnly for &Report {}
impl ReportOnly for &mut Report {}

/// Keeps [`ReportOnly`] closed: a `TrackedResult` over another error type would
/// have no `?`.
mod sealed {
    pub trait Sealed {}

    impl Sealed for crate::Report {}
    impl Sealed for &crate::Report {}
    impl Sealed for &mut crate::Report {}
}

/// The name to import when substituting [`TrackedResult`] for std `Result`.
pub use self::TrackedResult as Result;
pub use self::TrackedResult::{Err, Ok};

/// The methods below mirror std `Result`, in std's order. They do not care
/// whether the report is owned or borrowed.
impl<T, E: ReportOnly> TrackedResult<T, E> {
    /// Leave the tracked world, e.g. for an API bounded on std `Result`.
    pub fn into_std(self) -> StdResult<T, E> {
        match self {
            Ok(t) => StdResult::Ok(t),
            Err(e) => StdResult::Err(e),
        }
    }

    pub const fn is_ok(&self) -> bool {
        matches!(self, Ok(_))
    }

    pub fn is_ok_and<F>(self, f: F) -> bool
    where
        F: FnOnce(T) -> bool,
    {
        self.into_std().is_ok_and(f)
    }

    pub const fn is_err(&self) -> bool {
        matches!(self, Err(_))
    }

    pub fn is_err_and<F>(self, f: F) -> bool
    where
        F: FnOnce(E) -> bool,
    {
        self.into_std().is_err_and(f)
    }

    pub fn ok(self) -> Option<T> {
        self.into_std().ok()
    }

    pub fn err(self) -> Option<E> {
        self.into_std().err()
    }

    pub fn map<U, F>(self, f: F) -> TrackedResult<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Ok(t) => Ok(f(t)),
            Err(e) => Err(e),
        }
    }

    pub fn map_or<U, F>(self, default: U, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        self.into_std().map_or(default, f)
    }

    pub fn map_or_else<U, D, F>(self, default: D, f: F) -> U
    where
        D: FnOnce(E) -> U,
        F: FnOnce(T) -> U,
    {
        self.into_std().map_or_else(default, f)
    }

    pub fn map_or_default<U, F>(self, f: F) -> U
    where
        U: Default,
        F: FnOnce(T) -> U,
    {
        match self {
            Ok(t) => f(t),
            Err(_) => U::default(),
        }
    }

    pub fn inspect<F>(self, f: F) -> Self
    where
        F: FnOnce(&T),
    {
        if let Ok(t) = &self {
            f(t);
        }
        self
    }

    pub fn inspect_err<F>(self, f: F) -> Self
    where
        F: FnOnce(&E),
    {
        if let Err(e) = &self {
            f(e);
        }
        self
    }

    /// std has a dedicated `result::Iter`, which cannot be built from outside
    /// std; the `Option` iterator yields the same zero or one items.
    pub fn iter(&self) -> std::option::IntoIter<&T> {
        match self {
            Ok(t) => Some(t),
            Err(_) => None,
        }
        .into_iter()
    }

    pub fn iter_mut(&mut self) -> std::option::IntoIter<&mut T> {
        match self {
            Ok(t) => Some(t),
            Err(_) => None,
        }
        .into_iter()
    }

    #[track_caller]
    pub fn expect(self, msg: &str) -> T
    where
        E: std::fmt::Debug,
    {
        self.into_std().expect(msg)
    }

    #[track_caller]
    pub fn unwrap(self) -> T
    where
        E: std::fmt::Debug,
    {
        self.into_std().unwrap()
    }

    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        self.into_std().unwrap_or_default()
    }

    #[track_caller]
    pub fn expect_err(self, msg: &str) -> E
    where
        T: std::fmt::Debug,
    {
        self.into_std().expect_err(msg)
    }

    #[track_caller]
    pub fn unwrap_err(self) -> E
    where
        T: std::fmt::Debug,
    {
        self.into_std().unwrap_err()
    }

    pub fn and<U>(self, res: TrackedResult<U, E>) -> TrackedResult<U, E> {
        match self {
            Ok(_) => res,
            Err(e) => Err(e),
        }
    }

    pub fn and_then<U, F>(self, op: F) -> TrackedResult<U, E>
    where
        F: FnOnce(T) -> TrackedResult<U, E>,
    {
        match self {
            Ok(t) => op(t),
            Err(e) => Err(e),
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        self.into_std().unwrap_or(default)
    }

    pub fn unwrap_or_else<F>(self, op: F) -> T
    where
        F: FnOnce(E) -> T,
    {
        self.into_std().unwrap_or_else(op)
    }

    /// # Safety
    ///
    /// Calling this method on an `Err` is undefined behavior.
    pub unsafe fn unwrap_unchecked(self) -> T {
        // SAFETY: the caller guarantees `self` is `Ok`, which is std's contract.
        unsafe { self.into_std().unwrap_unchecked() }
    }

    /// # Safety
    ///
    /// Calling this method on an `Ok` is undefined behavior.
    pub unsafe fn unwrap_err_unchecked(self) -> E {
        // SAFETY: the caller guarantees `self` is `Err`, which is std's contract.
        unsafe { self.into_std().unwrap_err_unchecked() }
    }
}

/// The rest of std's `Result` methods need the report itself: the borrowing
/// views hand out `&Report`, and where std lets the error type change, the new
/// error is converted back into a `Report`.
impl<T> TrackedResult<T> {
    pub const fn as_ref(&self) -> TrackedResult<&T, &Report> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
        }
    }

    pub const fn as_mut(&mut self) -> TrackedResult<&mut T, &mut Report> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
        }
    }

    /// Whatever the closure returns is converted back into a `Report`, so the
    /// result stays tracked.
    #[track_caller]
    pub fn map_err<E, F>(self, f: F) -> TrackedResult<T>
    where
        E: IntoReport,
        F: FnOnce(Report) -> E,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(f(e).into_report()),
        }
    }

    pub fn as_deref(&self) -> TrackedResult<&T::Target, &Report>
    where
        T: std::ops::Deref,
    {
        self.as_ref().map(|t| &**t)
    }

    pub fn as_deref_mut(&mut self) -> TrackedResult<&mut T::Target, &mut Report>
    where
        T: std::ops::DerefMut,
    {
        self.as_mut().map(|t| &mut **t)
    }

    pub fn or(self, res: TrackedResult<T>) -> TrackedResult<T> {
        match self {
            Ok(t) => Ok(t),
            Err(_) => res,
        }
    }

    pub fn or_else<O>(self, op: O) -> TrackedResult<T>
    where
        O: FnOnce(Report) -> TrackedResult<T>,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => op(e),
        }
    }
}

/// erris's own result extensions ([`TrackReport`](crate::TrackReport),
/// [`WrapReport`](crate::WrapReport)), returning a `TrackedResult`.
impl<T> TrackedResult<T> {
    /// Add a location frame by hand. `?` records the hop itself, so this is
    /// needed only where a result is handed on without `?`; before a `?` on the
    /// same line it adds nothing. Kept so code moved over from std `Result`
    /// still compiles; the deprecation points at the calls to review.
    ///
    /// ```compile_fail
    /// #![deny(deprecated)]
    /// use erris::tracked::{Ok, Result};
    ///
    /// fn leaf() -> Result<u32> {
    ///     Ok(1)
    /// }
    ///
    /// fn hop() -> Result<u32> {
    ///     let n = leaf().track()?; // `?` alone records this hop
    ///     Ok(n)
    /// }
    /// ```
    #[deprecated(
        note = "usually redundant: `?` on a `TrackedResult` records the hop itself; keep \
                `.track()` only where the result is handed on without `?`"
    )]
    #[track_caller]
    pub fn track(self) -> TrackedResult<T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.track()),
        }
    }

    #[track_caller]
    pub fn wrap_report<M>(self, err: M) -> TrackedResult<T>
    where
        M: Into<Cow<'static, str>>,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.with_err(Report::from_message(err.into()))),
        }
    }

    #[track_caller]
    pub fn wrap_report_with<D, F>(self, f: F) -> TrackedResult<T>
    where
        D: IntoReport,
        F: FnOnce() -> D,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.with_err(f())),
        }
    }
}

/// For a std `Result<T, Report>` in tail position. No frame is added: `?` is
/// what tracks.
impl<T> From<StdResult<T, Report>> for TrackedResult<T> {
    fn from(result: StdResult<T, Report>) -> Self {
        match result {
            StdResult::Ok(t) => Ok(t),
            StdResult::Err(e) => Err(e),
        }
    }
}

impl<T, E: ReportOnly> TrackedResult<&T, E> {
    pub fn copied(self) -> TrackedResult<T, E>
    where
        T: Copy,
    {
        self.map(|&t| t)
    }

    pub fn cloned(self) -> TrackedResult<T, E>
    where
        T: Clone,
    {
        self.map(|t| t.clone())
    }
}

impl<T, E: ReportOnly> TrackedResult<&mut T, E> {
    pub fn copied(self) -> TrackedResult<T, E>
    where
        T: Copy,
    {
        self.map(|&mut t| t)
    }

    pub fn cloned(self) -> TrackedResult<T, E>
    where
        T: Clone,
    {
        self.map(|t| t.clone())
    }
}

impl<T, E: ReportOnly> TrackedResult<Option<T>, E> {
    pub fn transpose(self) -> Option<TrackedResult<T, E>> {
        match self {
            Ok(Some(t)) => Some(Ok(t)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl<T, E: ReportOnly> TrackedResult<TrackedResult<T, E>, E> {
    pub fn flatten(self) -> TrackedResult<T, E> {
        match self {
            Ok(inner) => inner,
            Err(e) => Err(e),
        }
    }
}

impl<T, E: ReportOnly> IntoIterator for TrackedResult<T, E> {
    type Item = T;
    type IntoIter = std::option::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.ok().into_iter()
    }
}

impl<'a, T, E: ReportOnly> IntoIterator for &'a TrackedResult<T, E> {
    type Item = &'a T;
    type IntoIter = std::option::IntoIter<&'a T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, E: ReportOnly> IntoIterator for &'a mut TrackedResult<T, E> {
    type Item = &'a mut T;
    type IntoIter = std::option::IntoIter<&'a mut T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T> Try for TrackedResult<T> {
    type Output = T;
    type Residual = TrackedResult<Infallible>;

    fn from_output(output: T) -> Self {
        Ok(output)
    }

    fn branch(self) -> ControlFlow<Self::Residual, T> {
        match self {
            Ok(t) => ControlFlow::Continue(t),
            Err(e) => ControlFlow::Break(Err(e)),
        }
    }
}

impl<T> Residual<T> for TrackedResult<Infallible> {
    type TryType = TrackedResult<T>;
}

/// What every `?` does to the report passing through it: make sure its top
/// frame is the `?` site. A report already located on that line — built there by
/// `wrap_report`, a `be` macro, or the error conversion itself — is left alone,
/// so the hop is recorded once.
#[track_caller]
fn located_here(report: Report) -> Report {
    let here = std::panic::Location::caller();
    let top = report.location();
    let same_place = top.file() == here.file() && top.line() == here.line();
    // `keep-duplicate-location` promises to keep frames that differ by column.
    #[cfg(feature = "keep-duplicate-location")]
    let same_place = same_place && top.column() == here.column();
    if same_place { report } else { report.track() }
}

/// `?` on a `TrackedResult` inside a function returning a `TrackedResult`.
impl<T> FromResidual<TrackedResult<Infallible>> for TrackedResult<T> {
    #[track_caller]
    fn from_residual(residual: TrackedResult<Infallible>) -> Self {
        match residual {
            Err(e) => Err(located_here(e)),
        }
    }
}

/// `?` on a std `Result` whose error a `Report` can be built from: a std error,
/// a `Report` itself, or a type with its own `From` impl.
impl<T, E> FromResidual<StdResult<Infallible, E>> for TrackedResult<T>
where
    Report: From<E>,
{
    #[track_caller]
    fn from_residual(residual: StdResult<Infallible, E>) -> Self {
        match residual {
            StdResult::Err(e) => Err(located_here(Report::from(e))),
        }
    }
}

/// `?` on a `TrackedResult` inside a function returning a std `Result`, e.g.
/// a handler whose error type is built from a `Report`.
impl<T, F> FromResidual<TrackedResult<Infallible>> for StdResult<T, F>
where
    F: From<Report>,
{
    #[track_caller]
    fn from_residual(residual: TrackedResult<Infallible>) -> Self {
        match residual {
            Err(e) => StdResult::Err(F::from(located_here(e))),
        }
    }
}

impl<T, V> FromIterator<TrackedResult<T>> for TrackedResult<V>
where
    V: FromIterator<T>,
{
    fn from_iter<I: IntoIterator<Item = TrackedResult<T>>>(iter: I) -> Self {
        let collected: StdResult<V, Report> =
            iter.into_iter().map(TrackedResult::into_std).collect();
        collected.into()
    }
}

impl<T, U> Sum<TrackedResult<U>> for TrackedResult<T>
where
    T: Sum<U>,
{
    fn sum<I: Iterator<Item = TrackedResult<U>>>(iter: I) -> Self {
        let total: StdResult<T, Report> = iter.map(TrackedResult::into_std).sum();
        total.into()
    }
}

impl<T, U> Product<TrackedResult<U>> for TrackedResult<T>
where
    T: Product<U>,
{
    fn product<I: Iterator<Item = TrackedResult<U>>>(iter: I) -> Self {
        let total: StdResult<T, Report> = iter.map(TrackedResult::into_std).product();
        total.into()
    }
}

impl<T: Termination> Termination for TrackedResult<T> {
    fn report(self) -> ExitCode {
        self.into_std().report()
    }
}
