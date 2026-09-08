# erris

`Report` is a container that wraps any error and builds a chain of causes as it
propagates, recording context and a caller location at each `?`. Its job is
introspection: instead of collapsing failures into one opaque value, it keeps the
whole chain walkable — and prints a readable message chain with the exact source
location of every link.

## Why not just [`eyre`]

erris covers the same job as `eyre` — one report type, `?`-friendly context — but
keeps the internals honest:

- **A real enum, no vtable magic.** The report is a concrete
  `enum ReportType { Transparent, Message, Report, ArcReport, Error, Wrapper }`,
  not a type-erased handler behind a hand-built vtable. Match on it, read it in a
  debugger, reason about its layout — the type tells the truth.
- **`wrap` / `track` take real errors, not anything that formats to a string.**
  Context methods are bounded on `std::error::Error` (via `IntoReport`), so a
  lossy `Display` / `Into<String>` value cannot silently become an error link.
- **Introspection-first.** The chain is walkable (`chain`, `unwrap_ref` /
  `unwrap_recursive`) and, with `to_json`, captured as JSON for structured
  logging — not just a `Display` blob.
- **`Location` instead of a backtrace by default.** Each link records a
  `&'static Location` via `#[track_caller]` — cheap, allocation-free, and it
  points at the propagation site instead of dumping a noisy stack. A full
  `Backtrace` stays available behind an opt-in feature.
- **Native `tracing` integration.** Each report captures the active span, and the
  `tracing_fields` layer records that span's field values into the report itself.
  `to_json` then emits the whole span chain — target, name, source location and
  captured fields — right in the log line, so the request id / user / route
  travels with the error instead of living in a separate log elsewhere.
- **Multiple `erris` versions coexist.** Reports cross a version boundary through
  the version-stable `erris-meta` trait (pinned to `1.x` forever, so its `TypeId`
  never splits): locations, spans, and both branches of every context wrapper
  survive, instead of collapsing into a bare `dyn Error` string mid-trace.

## Which method do I call?

The verb depends on what you are holding. Every one of them records a fresh
location via `#[track_caller]`.

| You have… | Call |
|-----------|------|
| a message or an error value | `report!` |
| a `Report` | `Report::with_message`, `with_report`, `with_err` |
| a `Result<T, E>` | `WrapReport::wrap_report`, `TrackReport::track`, `OkOrReport::ok_or_report` |
| an `Option<T>` | `OkOrReport::ok_or_report` |
| a `Result` with a boxed `dyn Error` | `WrapBoxReport::wrap_report` |

`with_*` operates on a report you already hold; the rest operate on the `Result`
or `Option` in the `?` position.

## Example

```rust
use erris::prelude::*;
use erris::report;

fn load() -> erris::Result<()> {
    let value: Result<(), std::io::Error> = Err(std::io::Error::other("disk gone"));
    value.wrap_report("could not load config")?;
    Ok(())
}
```

Debug output lists the message chain and the captured locations:

```text
could not load config

Caused by:
   0: disk gone

Location:
   0: src/config.rs:12:11
```

## Assertion macros — `be::*`

Check a value, get a `Result` instead of a panic:

```rust
use erris::be;

fn check(v: Option<u32>) -> erris::Result<u32> {
    let v = be::some!(v, "value is required")?;
    be::in_range!(v, 1..=100)?;
    Ok(v)
}
```

| Macro | `Ok` when | `Ok` value |
|-------|-----------|------------|
| `be::some!(opt)` | `Some(v)` | `v` (moved out) |
| `be::some_ref!(opt)` | `Some(v)` | `&v` |
| `be::none!(opt)` | `None` | `()` |
| `be::ok!(res)` | `Ok(v)` | `v` (moved out) |
| `be::err!(res)` | `Err(e)` | `e` (moved out) |
| `be::sure!(cond)` | the condition holds | `()` |
| `be::empty!(v)` / `be::non_empty!(v)` | `is_empty()` / `!is_empty()` | `()` |
| `be::zero!(v)` / `be::non_zero!(v)` | `== 0` / `!= 0` | `()` |
| `be::eq!(a, b)` / `be::ne!(a, b)` | `==` / `!=` | `()` |
| `be::lt!(a, b)` / `le!` / `gt!` / `ge!` | `<` / `<=` / `>` / `>=` | `()` |
| `be::matches!(v, Pat)` | the pattern matches | `()` |
| `be::contains!(hay, needle)` | `hay.contains(needle)` | `()` |
| `be::len!(v, n)` | `v.len() == n` | `()` |
| `be::in_range!(v, r)` | `r.contains(&v)` | `()` |

A macro that narrows a type hands back the narrowed value, a plain predicate
hands back `()`; the checked expression is evaluated exactly once. `be::ok!`
keeps the original error as the cause of the report instead of dropping it.

Every macro takes an optional message — a literal, a format string with
arguments, or any expression `report!` accepts. Without one the report names the
expression that failed:

```text
expected non zero: config.retries
expected timeout == expected, got 30 and 45
```

## Features

- `spantrace` _(default)_ — capture a `tracing` span trace per report link.
- `to_json` (implies `spantrace`) — `Report::to_json` plus the `tracing_fields`
  span-field layer.
- `backtrace` — capture a `std::backtrace::Backtrace`.
- `serde_json_value` (implies `to_json`) — inject a pre-serialised
  `serde_json::Value` into span fields via `JsonVisitor::record_json`.
- `keep-duplicate-location` — keep consecutive locations that differ only by
  column; by default they collapse into one entry per file and line.

Minimum supported Rust version: 1.86.

Full API documentation: [docs.rs/erris](https://docs.rs/erris).

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at
your option.

[`eyre`]: https://docs.rs/eyre
