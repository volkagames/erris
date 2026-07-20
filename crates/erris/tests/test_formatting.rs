//! Display and Debug rendering: the single-line message, the `Caused by:`
//! chain, location list, and transparent-report edge cases.

mod common;
use common::{Io, TestError};
use erris::*;

#[test]
fn display_is_the_top_message_only() {
    assert_eq!(report!("some error message").to_string(), "some error message");
    // a context wrapper displays its message, not the cause
    let wrapped = report!("the cause").with_message("the context");
    assert_eq!(wrapped.to_string(), "the context");
    // a report wrapping a report displays the inner message transparently
    assert_eq!(report!(report!("nested")).to_string(), "nested");
}

#[test]
fn message_macro_variants_render_equally() {
    // literal, owned String, and interpolation all produce the same message
    assert_eq!(report!("plain").to_string(), "plain");
    assert_eq!(report!("owned".to_string()).to_string(), "owned");
    let who = "world";
    assert_eq!(report!("hello {who}").to_string(), "hello world");
    let msg = "captured".to_string();
    assert_eq!(report!(msg).to_string(), "captured");
}

#[test]
fn debug_lists_causes_below_the_header() {
    let err = report!(Io("disk gone")).with_message("could not load config");
    let debug = format!("{err:?}");

    assert!(debug.starts_with("could not load config\n\nCaused by:"));
    assert!(debug.contains("io: disk gone"));
    // the header must not be repeated inside the cause list
    assert_eq!(debug.matches("could not load config").count(), 1);
}

#[test]
fn debug_chain_orders_causes_from_outer_to_inner() {
    let err = report!("root cause");
    let err = err.with_report(report!("middle"));
    let err = err.with_message("top");
    let debug = format!("{err:?}");

    let causes = debug
        .split("Caused by:")
        .nth(1)
        .and_then(|s| s.split("Location:").next())
        .unwrap();
    let top = debug.split("\n\n").next().unwrap();

    assert_eq!(top, "top");
    let middle = causes.find("middle").unwrap();
    let root = causes.find("root cause").unwrap();
    assert!(middle < root, "outer context should precede inner cause");
}

#[test]
fn transparent_report_has_no_dangling_caused_by() {
    let debug = format!("{:?}", report!());
    assert!(!debug.contains("Caused by:"));
}

#[test]
fn alternate_debug_renders_struct_fields() {
    let err = report!(TestError);
    let alt = format!("{err:#?}");
    // `{:#?}` uses the struct form, not the chain form
    assert!(alt.contains("ReportError"));
    assert!(alt.contains("location"));
    assert!(!alt.contains("Caused by:"));
}

#[test]
fn debug_includes_a_location() {
    let err = report!("boom");
    let debug = format!("{err:?}");
    assert!(debug.contains("Location:"));
    assert!(debug.contains("test_formatting.rs"));
}

/// Full-output snapshot of a deeply nested error touching every `ReportType`
/// node: a std error leaf, a `with_err` wrapper, a nested `report!(report)`,
/// and several message/report context layers.
///
/// The message header and the `Caused by:` chain are asserted verbatim. The
/// `Location:` block is checked structurally (count + which file each frame
/// points at) so the test survives line-number shifts. Spantrace/backtrace are
/// empty without an installed subscriber and are not part of the snapshot.
#[test]
fn debug_snapshot_of_deeply_nested_error() {
    let leaf = report!(Io("disk gone")); // Error(std)
    let with_std = leaf.with_err(TestError); // Wrapper(cause=Io, message=TestError)
    let nested = report!(with_std); // Report(nested)
    let ctx = nested.with_message("loading config"); // Wrapper
    let ctx2 = ctx.with_report(report!("stage: init")); // Wrapper
    let top = ctx2.with_message("startup failed"); // Wrapper
    let debug = format!("{top:?}");

    // Split off the stable head (header + causes) from the location tail.
    let (head, tail) = debug.split_once("\n\nLocation:").expect("has Location block");

    assert_eq!(
        head,
        "startup failed\n\n\
         Caused by:\n   \
         0: stage: init\n   \
         1: loading config\n   \
         2: TestError\n   \
         3: io: disk gone"
    );

    // The location block lists one frame per tracked construction point; assert
    // structure, not concrete line numbers.
    let location_block = tail.split("\n\nSpanTrace:").next().unwrap();
    let frames: Vec<&str> = location_block
        .lines()
        .filter(|l| l.contains("test_formatting.rs"))
        .collect();
    assert_eq!(frames.len(), 6, "one frame per tracked node");
    for (n, frame) in frames.iter().enumerate() {
        assert!(
            frame.trim_start().starts_with(&format!("{n}: ")),
            "frame {n} should be numbered and indented: {frame:?}"
        );
    }
}
