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
//! The error type defaults to [`Report`]. Another error type takes part by
//! implementing [`TrackedError`], which says how a `?` records its hop: an
//! HTTP layer's typed error, say, so that its handlers return
//! `erris::Result<T, ApiError>` and read the same `Ok`/`Err` as the code they
//! call. `?` passes such an error on unchanged and does not turn a `Report`
//! into it: that conversion stays explicit at the call site. The other way
//! round, an error of its own that is a std error becomes a `Report`, as it
//! would through a std `Result`.
//!
//! A function whose signature a foreign trait dictates keeps returning std
//! `Result`; `?` converts in both directions and tracks in both.
//!
//! The rest of erris follows: `track` and `wrap_report` are methods of this type
//! too, and the extension traits on std `Result` / `Option` and the
//! [`be`](crate::be) macros return it. A std `Result<T, Report>` from elsewhere
//! is consumed with `?`, or with `.into()` in tail position.
//!
//! With the `axum` feature, a `TrackedResult` whose value and error are both
//! responses is a response too, so an axum handler can return it directly.

use crate::{IntoReport, Report};
use std::borrow::Cow;
use std::convert::Infallible;
use std::iter::{Product, Sum};
use std::ops::{ControlFlow, FromResidual, Residual, Try};
use std::process::{ExitCode, Termination};
use std::result::Result as StdResult;

/// The error parameter keeps std's `Result<T, E>` shape, but it is not free: it
/// is a [`TrackedError`] — a [`Report`] unless named otherwise, owned or, as
/// [`as_ref`](TrackedResult::as_ref) and friends produce, borrowed. Naming an
/// error type that does not track is rejected with an explanation rather than
/// a bare trait error.
#[derive(Debug)]
#[must_use]
pub enum TrackedResult<T, E: TrackedError = Report> {
    Ok(T),
    Err(E),
}

/// An error type a [`TrackedResult`] can carry: one that records where a `?`
/// passed it on.
///
/// [`Report`] implements it, and so do borrows of any implementor (as a no-op,
/// since a borrowed error cannot be extended). A crate with its own error type
/// implements it to return `erris::Result<T, ThatError>`:
///
/// ```
/// use erris::tracked::{Err, Ok, Result, TrackedError};
/// use std::panic::Location;
///
/// #[derive(Debug)]
/// struct ApiError {
///     hops: Vec<&'static Location<'static>>,
/// }
///
/// impl TrackedError for ApiError {
///     #[track_caller]
///     fn track_hop(mut self) -> Self {
///         self.hops.push(Location::caller());
///         self
///     }
/// }
///
/// fn leaf() -> Result<u32, ApiError> {
///     Err(ApiError { hops: vec![] })
/// }
///
/// fn handler() -> Result<u32, ApiError> {
///     let n = leaf()?; // the hop is recorded here
///     Ok(n)
/// }
///
/// assert_eq!(handler().unwrap_err().hops.len(), 1);
/// ```
///
/// An error type that does not track is rejected:
///
/// ```compile_fail
/// fn load() -> erris::Result<(), std::io::Error> {
///     todo!()
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`erris::Result` needs an error type that tracks, and `{Self}` does not",
    label = "not an `erris::tracked::TrackedError`",
    note = "write `erris::Result<T>` for an `erris::Report`, implement `TrackedError` for \
            `{Self}`, or, for a result that is not tracked, spell out \
            `std::result::Result<T, {Self}>`"
)]
pub trait TrackedError: Sized {
    /// Record the hop of the `?` that is passing this error on: the caller's
    /// location. Called once per `?`, after any `From` conversion, under
    /// `#[track_caller]` — so an implementation marked `#[track_caller]` sees
    /// the `?` site as [`Location::caller`](std::panic::Location::caller).
    #[track_caller]
    fn track_hop(self) -> Self;
}

impl TrackedError for Report {
    /// Make sure the report's top frame is the `?` site. A report already
    /// located on that line — built there by `wrap_report`, a `be` macro, or the
    /// error conversion itself — is left alone, so the hop is recorded once.
    #[track_caller]
    fn track_hop(self) -> Self {
        let here = std::panic::Location::caller();
        let top = self.location();
        let same_place = top.file() == here.file() && top.line() == here.line();
        // `keep-duplicate-location` promises to keep frames that differ by column.
        #[cfg(feature = "keep-duplicate-location")]
        let same_place = same_place && top.column() == here.column();
        if same_place { self } else { self.track() }
    }
}

/// A borrowed error has nowhere to record the hop.
impl<E: TrackedError> TrackedError for &E {
    fn track_hop(self) -> Self {
        self
    }
}

/// A borrowed error has nowhere to record the hop.
impl<E: TrackedError> TrackedError for &mut E {
    fn track_hop(self) -> Self {
        self
    }
}

/// The name to import when substituting [`TrackedResult`] for std `Result`.
pub use self::TrackedResult as Result;
pub use self::TrackedResult::{Err, Ok};

/// The methods below mirror std `Result`, in std's order. They do not care
/// whether the report is owned or borrowed.
impl<T, E: TrackedError> TrackedResult<T, E> {
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

/// The rest of std's `Result` methods hand the error out: the borrowing views
/// as `&E`, and `map_err` to the closure, returning a std `Result` with the
/// closure's error type.
impl<T, E: TrackedError> TrackedResult<T, E> {
    pub const fn as_ref(&self) -> TrackedResult<&T, &E> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
        }
    }

    pub const fn as_mut(&mut self) -> TrackedResult<&mut T, &mut E> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
        }
    }

    /// Like std's: the error becomes whatever the closure returns, so the
    /// result is a std `Result<T, F>` — `res.map_err(|e| Foreign::from(e))`
    /// reads the same with and without `tracked`. A `?` on it still records
    /// the hop. To add context and stay tracked, use
    /// [`wrap_report`](Self::wrap_report) instead.
    pub fn map_err<F, O>(self, op: O) -> StdResult<T, F>
    where
        O: FnOnce(E) -> F,
    {
        match self {
            Ok(t) => StdResult::Ok(t),
            Err(e) => StdResult::Err(op(e)),
        }
    }

    pub fn as_deref(&self) -> TrackedResult<&T::Target, &E>
    where
        T: std::ops::Deref,
    {
        self.as_ref().map(|t| &**t)
    }

    pub fn as_deref_mut(&mut self) -> TrackedResult<&mut T::Target, &mut E>
    where
        T: std::ops::DerefMut,
    {
        self.as_mut().map(|t| &mut **t)
    }

    pub fn or(self, res: TrackedResult<T, E>) -> TrackedResult<T, E> {
        match self {
            Ok(t) => Ok(t),
            Err(_) => res,
        }
    }

    pub fn or_else<O>(self, op: O) -> TrackedResult<T, E>
    where
        O: FnOnce(E) -> TrackedResult<T, E>,
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
    /// Add a location frame by hand, where a result is handed on without `?`:
    /// a tail call or a `return`. Before a `?` it adds nothing, since `?` records
    /// the hop itself; `.track()?` left over from std `Result` code is harmless
    /// and can be dropped.
    ///
    /// ```
    /// use erris::tracked::{Ok, Result};
    ///
    /// fn leaf() -> Result<u32> {
    ///     Ok(1)
    /// }
    ///
    /// fn tail() -> Result<u32> {
    ///     leaf().track() // no `?` here, so the hop needs the frame by hand
    /// }
    /// # assert_eq!(tail().ok(), Some(1));
    /// ```
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

/// For a std `Result<T, E>` in tail position. No frame is added: `?` is what
/// tracks.
impl<T, E: TrackedError> From<StdResult<T, E>> for TrackedResult<T, E> {
    fn from(result: StdResult<T, E>) -> Self {
        match result {
            StdResult::Ok(t) => Ok(t),
            StdResult::Err(e) => Err(e),
        }
    }
}

impl<T, E: TrackedError> TrackedResult<&T, E> {
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

impl<T, E: TrackedError> TrackedResult<&mut T, E> {
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

impl<T, E: TrackedError> TrackedResult<Option<T>, E> {
    pub fn transpose(self) -> Option<TrackedResult<T, E>> {
        match self {
            Ok(Some(t)) => Some(Ok(t)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// `Option<TrackedResult<T>>` → `TrackedResult<Option<T>>`, the counterpart of
/// std's `Option::<Result<T, E>>::transpose`, which only takes a std `Result`.
///
/// `erris::prelude` exports it, so `opt.map(load).transpose()` reads the same
/// with and without `tracked`: std's inherent method does not apply to an
/// `Option<TrackedResult>`, and the call resolves to this trait instead.
pub trait OptionTranspose<T, E: TrackedError> {
    fn transpose(self) -> TrackedResult<Option<T>, E>;
}

impl<T, E: TrackedError> OptionTranspose<T, E> for Option<TrackedResult<T, E>> {
    fn transpose(self) -> TrackedResult<Option<T>, E> {
        match self {
            Some(Ok(t)) => Ok(Some(t)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }
}

impl<T, E: TrackedError> TrackedResult<TrackedResult<T, E>, E> {
    pub fn flatten(self) -> TrackedResult<T, E> {
        match self {
            Ok(inner) => inner,
            Err(e) => Err(e),
        }
    }
}

impl<T, E: TrackedError> IntoIterator for TrackedResult<T, E> {
    type Item = T;
    type IntoIter = std::option::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.ok().into_iter()
    }
}

impl<'a, T, E: TrackedError> IntoIterator for &'a TrackedResult<T, E> {
    type Item = &'a T;
    type IntoIter = std::option::IntoIter<&'a T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, E: TrackedError> IntoIterator for &'a mut TrackedResult<T, E> {
    type Item = &'a mut T;
    type IntoIter = std::option::IntoIter<&'a mut T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T, E: TrackedError> Try for TrackedResult<T, E> {
    type Output = T;
    type Residual = TrackedResult<Infallible, E>;

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

impl<T, E: TrackedError> Residual<T> for TrackedResult<Infallible, E> {
    type TryType = TrackedResult<T, E>;
}

/// `?` on a `TrackedResult` inside a function returning a `TrackedResult` with
/// the same error type. A `Report` is deliberately not turned into another
/// tracked error: it becomes, say, an HTTP error at the call site, where the
/// code and message are chosen. Keeping the error type fixed also lets an async
/// block's error type be inferred from its first `?`.
impl<T, E: TrackedError> FromResidual<TrackedResult<Infallible, E>> for TrackedResult<T, E> {
    #[track_caller]
    fn from_residual(residual: TrackedResult<Infallible, E>) -> Self {
        match residual {
            Err(e) => Err(e.track_hop()),
        }
    }
}

/// `?` on a `TrackedResult` over an error of its own that is a std error, such
/// as an HTTP layer's typed error, inside a function returning an erris
/// `Result`: the error becomes a `Report`, as it would through a std `Result`.
///
/// It does not overlap the impl above, since a `Report` is not a std error, and
/// a `?` on a `Report` still infers a `Report` for an async block or closure.
impl<T, E> FromResidual<TrackedResult<Infallible, E>> for TrackedResult<T>
where
    E: TrackedError + std::error::Error + Send + Sync + 'static,
{
    #[track_caller]
    fn from_residual(residual: TrackedResult<Infallible, E>) -> Self {
        match residual {
            Err(e) => Err(Report::from(e).track_hop()),
        }
    }
}

/// `?` on a std `Result` whose error the tracked error can be built from: for
/// a `Report`, a std error, a `Report` itself, or a type with its own `From`
/// impl.
impl<T, E, X> FromResidual<StdResult<Infallible, X>> for TrackedResult<T, E>
where
    E: TrackedError + From<X>,
{
    #[track_caller]
    fn from_residual(residual: StdResult<Infallible, X>) -> Self {
        match residual {
            StdResult::Err(x) => Err(E::from(x).track_hop()),
        }
    }
}

/// `?` on a `TrackedResult` inside a function returning a std `Result`, e.g.
/// a handler whose error type is built from a `Report`.
impl<T, E, F> FromResidual<TrackedResult<Infallible, E>> for StdResult<T, F>
where
    E: TrackedError,
    F: From<E>,
{
    #[track_caller]
    fn from_residual(residual: TrackedResult<Infallible, E>) -> Self {
        match residual {
            Err(e) => StdResult::Err(F::from(e.track_hop())),
        }
    }
}

impl<T, V, E: TrackedError> FromIterator<TrackedResult<T, E>> for TrackedResult<V, E>
where
    V: FromIterator<T>,
{
    fn from_iter<I: IntoIterator<Item = TrackedResult<T, E>>>(iter: I) -> Self {
        let collected: StdResult<V, E> = iter.into_iter().map(TrackedResult::into_std).collect();
        collected.into()
    }
}

impl<T, U, E: TrackedError> Sum<TrackedResult<U, E>> for TrackedResult<T, E>
where
    T: Sum<U>,
{
    fn sum<I: Iterator<Item = TrackedResult<U, E>>>(iter: I) -> Self {
        let total: StdResult<T, E> = iter.map(TrackedResult::into_std).sum();
        total.into()
    }
}

impl<T, U, E: TrackedError> Product<TrackedResult<U, E>> for TrackedResult<T, E>
where
    T: Product<U>,
{
    fn product<I: Iterator<Item = TrackedResult<U, E>>>(iter: I) -> Self {
        let total: StdResult<T, E> = iter.map(TrackedResult::into_std).product();
        total.into()
    }
}

impl<T: Termination, E: TrackedError + std::fmt::Debug> Termination for TrackedResult<T, E> {
    fn report(self) -> ExitCode {
        self.into_std().report()
    }
}

/// A handler can return a `TrackedResult` as it would a std `Result`: the value
/// or the error, whichever it holds, is the response.
#[cfg(feature = "axum")]
impl<T, E> axum_core::response::IntoResponse for TrackedResult<T, E>
where
    T: axum_core::response::IntoResponse,
    E: TrackedError + axum_core::response::IntoResponse,
{
    fn into_response(self) -> axum_core::response::Response {
        match self {
            Ok(t) => t.into_response(),
            Err(e) => e.into_response(),
        }
    }
}
