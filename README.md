# erris

An error-reporting library in the spirit of [`anyhow`] / [`eyre`]: a single
`Report` type that any error converts into, capturing the caller location (and,
with features, a span trace and backtrace) at each step.

## Which method do I call?

The verb depends on what you are holding.

| You have…                           | To build a report | To add context                                                              |
| ----------------------------------- | ----------------- | --------------------------------------------------------------------------- |
| a message / error value             | `report!`         | —                                                                           |
| a `Report`                          | —                 | `Report::with_message`, `with_report`, `with_err`                           |
| a `Result<T, E>`                    | —                 | `WrapReport::wrap_report`, `TrackReport::track`, `OkOrReport::ok_or_report` |
| an `Option<T>`                      | —                 | `OkOrReport::ok_or_report`                                                  |
| a `Result` with a boxed `dyn Error` | —                 | `WrapBoxReport::wrap_report`                                                |

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

[`anyhow`]: https://docs.rs/anyhow
[`eyre`]: https://docs.rs/eyre
