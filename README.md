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
`be::matches!` matches by value, exactly like `std::matches!`; pass `&v` to keep
the value and bind references. Call the macros as `be::name!` — a glob
`use erris::be::*` clashes with std's `matches!`.

Every macro takes an optional message — a literal, a format string with
arguments, or any expression `report!` accepts. Without one the report names the
expression that failed:

```text
expected non zero: config.retries
expected timeout == expected, got 30 and 45
```

A message of your own replaces the default one, operand values included. The
comparison macros still record the operands in a span, but they reach the
report only with `spantrace` on and a `tracing_error::ErrorLayer` subscriber
installed.

## Tracked results (nightly)

With std `Result`, a `?` that hands a `Report` on unchanged records nothing: the
hop needs an explicit `.track()?`. The `tracked` feature turns `erris::Result`
into `TrackedResult`, whose `?` records the location itself:

```rust
use erris::prelude::*; // `Ok`/`Err`: std's by default, the tracking ones with `tracked`

fn parse(s: &str) -> erris::Result<u32> {
    let n: u32 = s.parse()?; // std error -> Report, located here
    Ok(n)
}

fn double(s: &str) -> erris::Result<u32> {
    let n = parse(s)?; // located here too, with no `.track()`
    Ok(n * 2)
}
```

The same code builds without `tracked`, so turning the feature on or off
changes no source. There the `?` in `double` records nothing; write
`parse(s).track()?` to locate that hop by hand.

`TrackedResult` mirrors the methods of std `Result`, and its error is always a
`Report`. A function whose signature a foreign trait dictates — a web handler, a
`FromStr` impl — keeps returning std `Result`; `?` converts in both directions
and records the hop either way. Each hop is recorded once: a report already
located on the `?` line — by `wrap_report`, a `be` macro, or the error
conversion itself — gets no second frame. `wrap_report`, `ok_or_report`,
`track` and the `be` macros return `erris::Result`, so with `tracked` on they
return a `TrackedResult`, and `be::ok!` / `be::err!` accept one.

Before turning it on:

- **It needs nightly.** The hook is `try_trait_v2`, which is unstable. Tested
  with the nightlies of 2026-07-30 (1.99) and 2026-09-25 (1.100).
- **It is not additive.** Cargo unifies features across the build, so `tracked`
  changes `erris::Result` for every crate that depends on erris, not only yours.
  Enable it in an application, never in a library.
- **`Ok` and `Err` come from the prelude.** A module that globs `erris::prelude`
  gets the tracking `Ok`/`Err` with `tracked` and std's without it. A bare
  `Result<T, E>` stays std's; an erris result is `erris::Result<T>`.
- **`erris::Result<T, E>` with an error type is a compile error.** The error is
  fixed to `Report`.

### Enabling `tracked` in an application

1. Pin a nightly toolchain (`rust-toolchain.toml`) and enable the feature on the
   application crate only: `erris = { version = "3.1", features = ["tracked"] }`.
   Crates that ship erris-based traits may need their own `tracked` feature
   forwarding to `erris/tracked`.
2. Glob `use erris::prelude::*;` in every module that returns `erris::Result`.
3. Drop `.track()` before `?` in the application's own code — with `tracked` it
   adds nothing there:
   `perl -pi -e 's/\.track\(\)\?/?/g' $(git ls-files 'path/to/app/*.rs')`.
   Keep it where a result is handed on without `?` (a tail call, a `return`),
   and in code that also builds without `tracked`, such as a library, where it
   is what locates the hop.
4. Fix what the compiler reports:

| Error | Cause | Fix |
|-------|-------|-----|
| `E0308` expected `Result`, found `TrackedResult` (or back) inside a derive's expansion | a derive emits bare `Ok`/`Err` (`strum::EnumString`, `enum_dispatch`, …) in a module that globs the prelude | move the type to a module without the glob and re-export it |
| `E0308` in a `fmt::Display`, `FromStr`, serde or framework impl | std result in a module that globs the prelude | `std::result::Result::Ok(..)` / `Err(..)` |
| `E0599` no method `transpose` on `Option<TrackedResult<_>>` | the glob is missing | `use erris::prelude::*;` (brings `OptionTranspose`) |
| a std API wants `Result` (`try_join_all`, diesel `transaction`, `OnceCell::get_or_try_init`) | it is bounded on std `Result` | `.into_std()` |
| `erris::Result<T, E>` does not compile | the error is fixed to `Report` | `std::result::Result<T, E>` |

### Libraries

A library never enables `tracked`, but an application may turn it on for the
whole build, so a library that returns `erris::Result` compiles in both modes.
Write it the way the table above describes — the prelude glob, `erris::Result<T>`,
spelled-out std results, `into_std()` at std boundaries — and check both modes in
CI:

```sh
cargo check --all-targets
cargo +nightly check --all-targets --features erris/tracked
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
- `tracked` _(nightly only)_ — `erris::Result` becomes `TrackedResult`, whose `?`
  records a location on every hop, and the prelude's `Ok`/`Err` become its
  variants. See [Tracked results](#tracked-results-nightly).

Minimum supported Rust version: 1.86; `tracked` needs a nightly toolchain.

Full API documentation: [docs.rs/erris](https://docs.rs/erris).

## Upgrading from 3.0

- **The prelude always exports `Ok`/`Err`** — std's without `tracked`, so
  nothing changes there — and no longer exports `Result`: write
  `erris::Result<T>`, and a bare `Result<T, E>` in a module that globs the
  prelude is std's.
- **`TrackedResult::track` is no longer deprecated.** It is the way to record a
  hop handed on without `?`.
- **`TrackedResult::map_err` works like std's**: it returns a std
  `Result<T, F>` with the closure's error type, so mapping into a foreign
  error reads the same in both modes. A `?` on it still records the hop; to add
  context and stay tracked, use `wrap_report`.
- **New:** `OptionTranspose` (`.transpose()` on `Option<TrackedResult>`) and
  `IntoStd` (`.into_std()` on a std `Result<T, Report>`, `&Report` or
  `&mut Report`), both in the prelude.

## Upgrading from 2.x

- **`be::matches!` matches by value**, like `std::matches!`. Drop the `*` on
  `Copy` bindings in guards (`Some(n) if *n > 2` becomes `Some(n) if n > 2`) and
  on `&str` values (`be::matches!(*cmd, "a")` becomes `be::matches!(cmd, "a")`).
  Where the pattern binds part of a value that has to stay in place, pass a
  reference: `be::matches!(&state, Some(s) if s.is_empty())`.
- **`be::ok!` adds no link for a `Report` error.** The error is now the cause as
  is, as with `wrap_report`, so its chain is one link shorter than in 2.x.
- **Features, as since 2.2.0:** `to_json` is not a default — enable it if you
  call `Report::to_json`; `valuable` is gone.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at
your option.

[`eyre`]: https://docs.rs/eyre
