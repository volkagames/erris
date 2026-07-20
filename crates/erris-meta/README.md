# erris-meta

Version-stable shared trait for [`erris`], enabling cross-version metadata
access between different `erris` builds in the same binary.

## Why this crate exists

When two different `erris` versions coexist in one dependency graph, each has
its own `ReportError` type with a distinct [`TypeId`]. As a result,
`downcast_ref::<ReportError>` fails across the version boundary: a newer
`erris` cannot recognize a report produced by an older one, and vice versa.

`erris-meta` provides a single trait — [`ReportMeta`] — whose `TypeId` is
identical across every `erris` build. Routing metadata access through this
shared trait instead of the per-version concrete type lets any `erris` version
read and traverse another version's report.

## The trait

```rust
pub trait ReportMeta: Error + Send + Sync + 'static {
    /// Location captured when this report link was created.
    fn report_location(&self) -> Option<&'static Location<'static>>;

    /// Whether this link is a transparent (message-less) wrapper.
    fn report_is_transparent(&self) -> bool;

    /// The child links of this report, as `dyn ReportMeta`.
    /// `(None, None)` for a leaf.
    fn report_branch(&self) -> (Option<&dyn ReportMeta>, Option<&dyn ReportMeta>);

    /// SpanTrace captured for this report link, if any.
    #[cfg(feature = "spantrace")]
    fn report_spantrace(&self) -> Option<&tracing_error::SpanTrace>;
}
```

`Error` is a supertrait, so a `&dyn ReportMeta` can be upcast to `&dyn Error` on
stable (upcasting stabilized in Rust 1.86). This provides `Display` and
`source()` without storing a separate pointer.

`report_branch` returns children through the shared trait rather than through
`dyn Error::source`, so a report built by a different `erris` version can still
be traversed with both branches intact — for example, a context wrapper's
`(cause, message)` pair.

## Semver contract

**This crate MUST stay on `1.x` forever.**

Changing the [`ReportMeta`] signature — or bumping to `2.0` — would split the
`TypeId` again and defeat the entire purpose of the crate. Add capability only
via new default methods; never alter existing ones.

## Features

| Feature     | Effect                                                                          |
| ----------- | ------------------------------------------------------------------------------- |
| `spantrace` | Adds `report_spantrace`, exposing a captured [`tracing_error::SpanTrace`]. Enables the `tracing-error` dependency. |

No features are enabled by default.

## License

Licensed under either of Apache-2.0 or MIT at your option, matching [`erris`].

[`erris`]: ../erris
[`ReportMeta`]: src/lib.rs
[`TypeId`]: https://doc.rust-lang.org/std/any/struct.TypeId.html
[`tracing_error::SpanTrace`]: https://docs.rs/tracing-error/latest/tracing_error/struct.SpanTrace.html
