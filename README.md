# erris

`Report` is a container that wraps any error and builds a chain of causes as it
propagates, recording context and a caller location at each `?`. Its job is
introspection: instead of collapsing failures into one opaque value, it keeps the
whole chain walkable — and prints a readable message chain with the exact source
location of every link.

## Why not just `eyre`

erris covers the same job as `eyre` — one report type, `?`-friendly context — but
keeps the internals honest:

- **A real enum, no vtable magic.** The report is a concrete
  `enum ReportType { Transparent, Message, Report, ArcReport, Error, Wrapper }`,
  not a type-erased handler behind a hand-built vtable. You can match on it,
  read it in a debugger, and reason about layout — the type tells the truth.
- **`wrap`/`track` take real errors, not anything that formats to a string.** Context methods
  are bounded on `std::error::Error` (via `IntoReport`), so a lossy
  `Display`/`Into<String>` value can't silently become an error link. No
  surprise stack loss from a value that merely happened to format.
- **Introspection-first.** The chain is walkable (`chain`, `unwrap_ref` /
  `unwrap_recursive`) and, with the `to_json` feature, captured as JSON for
  structured logging and parsing — not just a `Display` blob.
- **`Location` instead of a backtrace by default.** Each link records a
  `&'static Location` via `#[track_caller]` — cheap, allocation-free, and it
  points at the propagation site instead of dumping a noisy stack. A full
  `std::backtrace::Backtrace` stays available behind an opt-in feature.
- **Native `tracing` integration.** Each report captures the active span at the
  point it is created, and a `tracing_fields` layer records each span's field
  values so they land in the report itself. `to_json` then emits the full span
  chain — target, name, source location, and captured fields — right in the log
  line. That context (request id, user, route, …) travels with the error
  instead of living in a separate log elsewhere — especially handy in branchy
  applications and web servers where the failing path is otherwise hard to
  reconstruct.
- **Multiple `erris` versions coexist.** When two `erris` versions end up in one
  dependency graph, reports cross the boundary through the version-stable
  `erris-meta` trait (pinned to `1.x` forever, so its `TypeId` never splits).
  A report built by one version stays fully inspectable in another — locations,
  spans, and both branches of every context wrapper survive — instead of
  collapsing into a bare `dyn Error` string somewhere in the middle of the
  trace.

## Which method do I call?

The verb depends on what you are holding.

**Building a report**

- a message or an error value — `report!`

**Adding context**

- a `Report` you already hold — `Report::with_message`, `with_report`, `with_err`
- a `Result<T, E>` — `WrapReport::wrap_report`, `TrackReport::track`,
  `OkOrReport::ok_or_report`
- an `Option<T>` — `OkOrReport::ok_or_report`
- a `Result` with a boxed `dyn Error` — `WrapBoxReport::wrap_report`

`with_*` operates on a `Report` you already hold; `wrap_report` / `track` /
`ok_or_report` operate on the `Result` / `Option` in the `?` position. Both
record a fresh location via `#[track_caller]`.

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

## Features

- `spantrace` _(default)_ — capture a `tracing` span trace per report link.
- `to_json` _(default, implies `spantrace`)_ — `Report::to_json` plus the
  `tracing_fields` span-field layer.
- `backtrace` — capture a `std::backtrace::Backtrace`.
- `valuable` — record `valuable` span fields into JSON.

Full API documentation is in the crate-level docs (`cargo doc --open`).

[`eyre`]: https://docs.rs/eyre
