//! Assertion-style macros: check a value, get an [`crate::Result`] instead of a
//! panic.
//!
//! ```
//! # #[cfg(feature = "tracked")] use erris::tracked::Ok;
//! use erris::be;
//!
//! fn check(v: Option<u32>) -> erris::Result<()> {
//!     let v = be::some!(v, "value is required")?;
//!     be::non_zero!(v, "value must be non zero")?;
//!     Ok(())
//! }
//!
//! assert!(check(None).is_err());
//! assert!(check(Some(42)).is_ok());
//! ```
//!
//! Every macro takes an optional message — a literal, a format string with
//! arguments, or any expression [`report!`](crate::report) accepts. Without one,
//! the report names the checked expression:
//!
//! ```
//! use erris::be;
//!
//! let retries = 0;
//! assert_eq!(
//!     be::non_zero!(retries).unwrap_err().to_string(),
//!     "expected non zero: retries",
//! );
//! ```
//!
//! # What each macro returns
//!
//! A macro that *narrows* a type yields the narrowed value; a plain predicate
//! yields `()`.
//!
//! | Macro | `Ok` when | `Ok` value |
//! |-------|-----------|------------|
//! | [`some!`](crate::be::some) | `Some(v)` | `v` (moved out) |
//! | [`some_ref!`](crate::be::some_ref) | `Some(v)` | `&v` |
//! | [`none!`](crate::be::none) | `None` | `()` |
//! | [`ok!`](crate::be::ok) | `Ok(v)` | `v` (moved out) |
//! | [`err!`](crate::be::err) | `Err(e)` | `e` (moved out) |
//! | [`sure!`](crate::be::sure) | the condition holds | `()` |
//! | [`empty!`](crate::be::empty) / [`non_empty!`](crate::be::non_empty) | `is_empty()` / `!is_empty()` | `()` |
//! | [`zero!`](crate::be::zero) / [`non_zero!`](crate::be::non_zero) | `== 0` / `!= 0` | `()` |
//! | [`eq!`](crate::be::eq) / [`ne!`](crate::be::ne) | `==` / `!=` | `()` |
//! | [`lt!`](crate::be::lt) / [`le!`](crate::be::le) / [`gt!`](crate::be::gt) / [`ge!`](crate::be::ge) | `<` / `<=` / `>` / `>=` | `()` |
//! | [`matches!`](crate::be::matches) | the pattern matches | `()` |
//! | [`contains!`](crate::be::contains) | `haystack.contains(needle)` | `()` |
//! | [`len!`](crate::be::len) | `v.len() == n` | `()` |
//! | [`in_range!`](crate::be::in_range) | `range.contains(&v)` | `()` |
//!
//! The checked expression is evaluated exactly once, so arguments with side
//! effects are safe.
//!
//! [`ok!`](crate::be::ok) and [`err!`](crate::be::err) take the result by value.
//! To check one with `err!` that has to stay where it is, pass a reference and
//! get references back. `ok!` needs the result by value: its error becomes
//! the cause of the report, and a report cannot hold a borrowed error.
//!
//! ```
//! use erris::be;
//!
//! struct Job {
//!     outcome: Result<String, u8>,
//! }
//! let job = Job { outcome: Err(7) };
//!
//! let code: &u8 = be::err!(&job.outcome).unwrap();
//! assert_eq!(*code, 7);
//! ```
//!
//! The comparison macros require [`Debug`](std::fmt::Debug) on both operands,
//! like [`assert_eq!`]. The values go into the default message. A caller's own
//! message replaces it, so then the values reach the report only through the
//! span the macro enters — which takes the `spantrace` feature *and* a
//! subscriber with [`tracing_error::ErrorLayer`](https://docs.rs/tracing-error)
//! installed. Without both, put the values in the message yourself.
//!
//! Call the macros through the module path — `be::eq!`, not a glob import:
//! `use erris::be::*` brings in a `matches!` that clashes with the std one.

#[doc(inline)]
pub use crate::{
    __be_contains as contains,
    __be_empty as empty,
    __be_eq as eq,
    __be_err as err,
    __be_ge as ge,
    __be_gt as gt,
    __be_in_range as in_range,
    __be_le as le,
    __be_len as len,
    __be_lt as lt,
    __be_matches as matches,
    __be_ne as ne,
    __be_non_empty as non_empty,
    __be_non_zero as non_zero,
    __be_none as none,
    __be_ok as ok,
    __be_some as some,
    __be_some_ref as some_ref,
    __be_sure as sure,
    __be_zero as zero,
};

/// What [`ok!`](crate::be::ok) and [`err!`](crate::be::err) accept: a std
/// `Result` and, with the `tracked` feature, a `TrackedResult` — by value or by
/// reference. A reference yields references, as matching on `&result` does.
#[doc(hidden)]
pub trait __BeResult {
    type Ok;
    type Err;

    fn __be_into_std(self) -> std::result::Result<Self::Ok, Self::Err>;
}

impl<T, E> __BeResult for std::result::Result<T, E> {
    type Err = E;
    type Ok = T;

    fn __be_into_std(self) -> std::result::Result<T, E> {
        self
    }
}

impl<'a, T, E> __BeResult for &'a std::result::Result<T, E> {
    type Err = &'a E;
    type Ok = &'a T;

    fn __be_into_std(self) -> std::result::Result<&'a T, &'a E> {
        self.as_ref()
    }
}

impl<'a, T, E> __BeResult for &'a mut std::result::Result<T, E> {
    type Err = &'a mut E;
    type Ok = &'a mut T;

    fn __be_into_std(self) -> std::result::Result<&'a mut T, &'a mut E> {
        self.as_mut()
    }
}

#[cfg(feature = "tracked")]
impl<T> __BeResult for crate::TrackedResult<T> {
    type Err = crate::Report;
    type Ok = T;

    fn __be_into_std(self) -> std::result::Result<T, crate::Report> {
        self.into_std()
    }
}

#[cfg(feature = "tracked")]
impl<'a, T> __BeResult for &'a crate::TrackedResult<T> {
    type Err = &'a crate::Report;
    type Ok = &'a T;

    fn __be_into_std(self) -> std::result::Result<&'a T, &'a crate::Report> {
        self.as_ref().into_std()
    }
}

#[cfg(feature = "tracked")]
impl<'a, T> __BeResult for &'a mut crate::TrackedResult<T> {
    type Err = &'a mut crate::Report;
    type Ok = &'a mut T;

    fn __be_into_std(self) -> std::result::Result<&'a mut T, &'a mut crate::Report> {
        self.as_mut().into_std()
    }
}

/// Build the failure report: the caller's message when there is one, the
/// macro's own description of the failed check otherwise.
#[doc(hidden)]
#[macro_export]
macro_rules! __be_report {
    ($default:expr $(,)?) => {
        $crate::report!($default)
    };
    ($default:expr, $($arg:tt)+) => {
        $crate::report!($($arg)+)
    };
}

/// Same, but for the comparison macros: the operands are recorded in a span so
/// they reach the report even when the caller supplied its own message.
#[cfg(feature = "spantrace")]
#[doc(hidden)]
#[macro_export]
macro_rules! __be_cmp_report {
    ($name:literal, $left:expr, $right:expr, $($rest:tt)+) => {{
        let __be_span = $crate::__tracing::error_span!($name, left = ?$left, right = ?$right);
        let __be_entered = __be_span.enter();
        $crate::__be_report!($($rest)+)
    }};
}

#[cfg(not(feature = "spantrace"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __be_cmp_report {
    ($name:literal, $left:expr, $right:expr, $($rest:tt)+) => {
        $crate::__be_report!($($rest)+)
    };
}

/// Shared body of [`eq!`](crate::be::eq) and its siblings.
#[doc(hidden)]
#[macro_export]
macro_rules! __be_cmp {
    ($name:literal, $op:tt, $left:expr, $right:expr $(, $($arg:tt)*)?) => {
        match (&$left, &$right) {
            (__be_left, __be_right) => {
                if *__be_left $op *__be_right {
                    $crate::Result::Ok(())
                } else {
                    // Both operands are Debug-formatted into the default message
                    // and into the span. Assert the bound here so enabling
                    // `spantrace` cannot add a bound a dependent build lacks.
                    fn __be_assert_debug<T: ?Sized + ::core::fmt::Debug>(_: &T) {}
                    __be_assert_debug(__be_left);
                    __be_assert_debug(__be_right);
                    $crate::Result::Err($crate::__be_cmp_report!(
                        $name,
                        __be_left,
                        __be_right,
                        // The operands' source text goes in as arguments, never
                        // into the format string: `{` in a struct literal or a
                        // string operand would be read as a placeholder.
                        ::std::format!(
                            "expected {} {} {}, got {:?} and {:?}",
                            ::core::stringify!($left),
                            ::core::stringify!($op),
                            ::core::stringify!($right),
                            __be_left,
                            __be_right,
                        )
                        $(, $($arg)*)?
                    ))
                }
            }
        }
    };
}

/// `Ok(v)` when the option is `Some(v)`, moving the value out.
///
/// ```
/// # #[cfg(feature = "tracked")] use erris::tracked::Ok;
/// use erris::be;
///
/// fn parse(raw: Option<&str>) -> erris::Result<u32> {
///     let raw = be::some!(raw, "no input")?;
///     Ok(raw.len() as u32)
/// }
///
/// assert_eq!(parse(Some("abc")).unwrap(), 3);
/// assert_eq!(parse(None).unwrap_err().to_string(), "no input");
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_some {
    ($e:expr $(, $($arg:tt)*)?) => {
        match $e {
            ::core::option::Option::Some(__be_value) => $crate::Result::Ok(__be_value),
            ::core::option::Option::None => $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected Some: ", ::core::stringify!($e)) $(, $($arg)*)?
            )),
        }
    };
}

/// `Ok(&v)` when the option is `Some(v)` — the borrowing form of
/// [`some!`](crate::be::some), for when the option must stay in place.
///
/// ```
/// use erris::be;
///
/// struct Config { name: Option<String> }
/// let config = Config { name: Some("erris".into()) };
///
/// let name: &String = be::some_ref!(config.name).unwrap();
/// assert_eq!(name, "erris");
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_some_ref {
    ($e:expr $(, $($arg:tt)*)?) => {
        match &$e {
            ::core::option::Option::Some(__be_value) => $crate::Result::Ok(__be_value),
            ::core::option::Option::None => $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected Some: ", ::core::stringify!($e)) $(, $($arg)*)?
            )),
        }
    };
}

/// `Ok(())` when the option is `None`.
///
/// ```
/// use erris::be;
///
/// assert!(be::none!(Option::<u8>::None).is_ok());
/// assert_eq!(
///     be::none!(Some(1)).unwrap_err().to_string(),
///     "expected None: Some(1)",
/// );
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_none {
    ($e:expr $(, $($arg:tt)*)?) => {
        match &$e {
            ::core::option::Option::Some(_) => $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected None: ", ::core::stringify!($e)) $(, $($arg)*)?
            )),
            ::core::option::Option::None => $crate::Result::Ok(()),
        }
    };
}

/// `Ok(v)` when the result is `Ok(v)`. The error is not thrown away: it becomes
/// the cause of the reported failure, as with
/// [`wrap_report`](crate::WrapReport::wrap_report). A [`Report`](crate::Report)
/// error is kept as is, without an extra link. The error can be anything
/// [`report!`](crate::report) accepts, so unlike `wrap_report` a `String` or
/// `&'static str` error works too and becomes a message link.
///
/// ```
/// use erris::be;
///
/// let failed: Result<(), std::io::Error> = Err(std::io::Error::other("disk gone"));
/// let report = be::ok!(failed, "could not load config").unwrap_err();
///
/// assert_eq!(report.to_string(), "could not load config");
/// // the io error is still in the chain, it did not collapse into the message
/// assert!(report.chain().any(|link| link.as_error().to_string() == "disk gone"));
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_ok {
    ($e:expr $(, $($arg:tt)*)?) => {
        match $crate::be::__BeResult::__be_into_std($e) {
            ::core::result::Result::Ok(__be_value) => $crate::Result::Ok(__be_value),
            ::core::result::Result::Err(__be_error) => $crate::Result::Err({
                use $crate::report_kind::*;
                (&__be_error).report_kind().into_cause(__be_error)
            }
            .with_err($crate::__be_report!(
                ::core::concat!("expected Ok: ", ::core::stringify!($e)) $(, $($arg)*)?
            ))),
        }
    };
}

/// `Ok(e)` when the result is `Err(e)`, moving the error out.
///
/// ```
/// use erris::be;
///
/// let failed: Result<(), &str> = Err("boom");
/// assert_eq!(be::err!(failed).unwrap(), "boom");
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_err {
    ($e:expr $(, $($arg:tt)*)?) => {
        match $crate::be::__BeResult::__be_into_std($e) {
            ::core::result::Result::Err(__be_error) => $crate::Result::Ok(__be_error),
            ::core::result::Result::Ok(_) => $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected Err: ", ::core::stringify!($e)) $(, $($arg)*)?
            )),
        }
    };
}

/// `Ok(())` when the condition holds.
///
/// ```
/// use erris::be;
///
/// let port = 0;
/// assert_eq!(
///     be::sure!(port > 0).unwrap_err().to_string(),
///     "expected true: port > 0",
/// );
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_sure {
    ($e:expr $(, $($arg:tt)*)?) => {
        if $e {
            $crate::Result::Ok(())
        } else {
            $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected true: ", ::core::stringify!($e)) $(, $($arg)*)?
            ))
        }
    };
}

/// `Ok(())` when `is_empty()` holds — for any type with that inherent method.
///
/// ```
/// use erris::be;
///
/// assert!(be::empty!(Vec::<u8>::new()).is_ok());
/// assert!(be::empty!("text").is_err());
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_empty {
    ($e:expr $(, $($arg:tt)*)?) => {{
        let __be_value = &$e;
        if __be_value.is_empty() {
            $crate::Result::Ok(())
        } else {
            $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected empty: ", ::core::stringify!($e)) $(, $($arg)*)?
            ))
        }
    }};
}

/// `Ok(())` when `is_empty()` does not hold.
///
/// ```
/// use erris::be;
///
/// assert!(be::non_empty!("text").is_ok());
/// assert!(be::non_empty!("").is_err());
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_non_empty {
    ($e:expr $(, $($arg:tt)*)?) => {{
        let __be_value = &$e;
        if !__be_value.is_empty() {
            $crate::Result::Ok(())
        } else {
            $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected non empty: ", ::core::stringify!($e)) $(, $($arg)*)?
            ))
        }
    }};
}

/// `Ok(())` when the value equals zero. The value is compared with the integer
/// literal `0`, so this works for integers; for floats use
/// [`sure!`](crate::be::sure).
///
/// ```
/// use erris::be;
///
/// assert!(be::zero!(0_u8).is_ok());
/// assert!(be::zero!(1_u8).is_err());
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_zero {
    ($e:expr $(, $($arg:tt)*)?) => {{
        let __be_value = &$e;
        if *__be_value == 0 {
            $crate::Result::Ok(())
        } else {
            $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected zero: ", ::core::stringify!($e)) $(, $($arg)*)?
            ))
        }
    }};
}

/// `Ok(())` when the value differs from zero. Integers only, as with
/// [`zero!`](crate::be::zero).
///
/// ```
/// use erris::be;
///
/// assert!(be::non_zero!(42_u64).is_ok());
/// assert!(be::non_zero!(0_i32).is_err());
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_non_zero {
    ($e:expr $(, $($arg:tt)*)?) => {{
        let __be_value = &$e;
        if *__be_value != 0 {
            $crate::Result::Ok(())
        } else {
            $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected non zero: ", ::core::stringify!($e)) $(, $($arg)*)?
            ))
        }
    }};
}

/// `assert_eq!` without the panic: `Ok(())` when both sides are equal.
///
/// ```
/// use erris::be;
///
/// assert_eq!(
///     be::eq!(1 + 1, 3).unwrap_err().to_string(),
///     "expected 1 + 1 == 3, got 2 and 3",
/// );
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_eq {
    ($left:expr, $right:expr $(, $($arg:tt)*)?) => {
        $crate::__be_cmp!("be::eq", ==, $left, $right $(, $($arg)*)?)
    };
}

/// `Ok(())` when both sides differ.
#[doc(hidden)]
#[macro_export]
macro_rules! __be_ne {
    ($left:expr, $right:expr $(, $($arg:tt)*)?) => {
        $crate::__be_cmp!("be::ne", !=, $left, $right $(, $($arg)*)?)
    };
}

/// `Ok(())` when `left < right`.
#[doc(hidden)]
#[macro_export]
macro_rules! __be_lt {
    ($left:expr, $right:expr $(, $($arg:tt)*)?) => {
        $crate::__be_cmp!("be::lt", <, $left, $right $(, $($arg)*)?)
    };
}

/// `Ok(())` when `left <= right`.
#[doc(hidden)]
#[macro_export]
macro_rules! __be_le {
    ($left:expr, $right:expr $(, $($arg:tt)*)?) => {
        $crate::__be_cmp!("be::le", <=, $left, $right $(, $($arg)*)?)
    };
}

/// `Ok(())` when `left > right`.
#[doc(hidden)]
#[macro_export]
macro_rules! __be_gt {
    ($left:expr, $right:expr $(, $($arg:tt)*)?) => {
        $crate::__be_cmp!("be::gt", >, $left, $right $(, $($arg)*)?)
    };
}

/// `Ok(())` when `left >= right`.
#[doc(hidden)]
#[macro_export]
macro_rules! __be_ge {
    ($left:expr, $right:expr $(, $($arg:tt)*)?) => {
        $crate::__be_cmp!("be::ge", >=, $left, $right $(, $($arg)*)?)
    };
}

/// `Ok(())` when the value matches the pattern. Matches exactly as
/// [`std::matches!`] does, guard included.
///
/// ```
/// use erris::be;
///
/// let state = Some(3);
/// assert!(be::matches!(state, Some(n) if n > 2).is_ok());
/// assert!(be::matches!(state, None).is_err());
///
/// let cmd: &str = "start";
/// assert!(be::matches!(cmd, "start" | "stop").is_ok());
/// ```
///
/// The value is matched by value, so a pattern that binds a non-`Copy` part
/// moves it out. Pass a reference to keep the value and bind references:
///
/// ```
/// use erris::be;
///
/// let name = Some(String::from("erris"));
/// assert!(be::matches!(&name, Some(n) if n.starts_with("err")).is_ok());
/// assert!(name.is_some()); // still ours
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_matches {
    ($e:expr, $pat:pat $(if $guard:expr)? $(, $($arg:tt)*)?) => {
        match $e {
            $pat $(if $guard)? => $crate::Result::Ok(()),
            _ => $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected ", ::core::stringify!($e), " to match ", ::core::stringify!($pat))
                $(, $($arg)*)?
            )),
        }
    };
}

/// `Ok(())` when `haystack.contains(needle)` holds. The needle is passed to the
/// inherent method untouched, so it follows that method's convention — a
/// pattern for `str`, a reference for slices and sets.
///
/// ```
/// use erris::be;
///
/// assert!(be::contains!("hello world", "world").is_ok());
/// assert!(be::contains!(vec![1, 2, 3], &2).is_ok());
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_contains {
    ($haystack:expr, $needle:expr $(, $($arg:tt)*)?) => {{
        let __be_haystack = &$haystack;
        if __be_haystack.contains($needle) {
            $crate::Result::Ok(())
        } else {
            $crate::Result::Err($crate::__be_report!(
                ::core::concat!(
                    "expected ", ::core::stringify!($haystack), " to contain ", ::core::stringify!($needle),
                )
                $(, $($arg)*)?
            ))
        }
    }};
}

/// `Ok(())` when `len()` equals the expected length. The actual length lands in
/// the default message.
///
/// ```
/// use erris::be;
///
/// assert_eq!(
///     be::len!("abc", 5).unwrap_err().to_string(),
///     r#"expected "abc" to have length 5, got 3"#,
/// );
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_len {
    ($e:expr, $len:expr $(, $($arg:tt)*)?) => {{
        let __be_actual = (&$e).len();
        let __be_expected = $len;
        if __be_actual == __be_expected {
            $crate::Result::Ok(())
        } else {
            $crate::Result::Err($crate::__be_report!(
                ::std::format!(
                    "expected {} to have length {}, got {}",
                    ::core::stringify!($e),
                    __be_expected,
                    __be_actual,
                )
                $(, $($arg)*)?
            ))
        }
    }};
}

/// `Ok(())` when the range contains the value. Any type with a `contains`
/// method works — `Range`, `RangeInclusive`, `RangeTo`, …
///
/// ```
/// use erris::be;
///
/// assert!(be::in_range!(7, 1..=10).is_ok());
/// assert_eq!(
///     be::in_range!(42, 1..=10).unwrap_err().to_string(),
///     "expected 42 in 1..=10",
/// );
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __be_in_range {
    ($e:expr, $range:expr $(, $($arg:tt)*)?) => {{
        let __be_value = &$e;
        if $range.contains(__be_value) {
            $crate::Result::Ok(())
        } else {
            $crate::Result::Err($crate::__be_report!(
                ::core::concat!("expected ", ::core::stringify!($e), " in ", ::core::stringify!($range))
                $(, $($arg)*)?
            ))
        }
    }};
}
