//! The `Report::chain` iterator and the `ReportLink` view it yields: link
//! ordering, location/transparency accessors, and traversal into both branches
//! of a context wrapper.

mod common;
use common::{Io, TestError};
use erris::*;

#[test]
fn chain_of_a_leaf_yields_one_link() {
    let report = report!("solo");
    let links: Vec<_> = report.chain().collect();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].as_error().to_string(), "solo");
}

#[test]
fn chain_visits_both_branches_of_a_wrapper() {
    // Wrapper(cause = Io, message = "context"): chain must reach the message
    // branch and the cause branch, plus the wrapper root itself.
    let report = report!(Io("disk gone")).with_message("context");

    let rendered: Vec<String> = report
        .chain()
        .map(|link| link.as_error().to_string())
        .collect();

    assert!(rendered.iter().any(|s| s == "context"), "message branch: {rendered:?}");
    assert!(rendered.iter().any(|s| s == "io: disk gone"), "cause branch: {rendered:?}");
}

#[test]
fn chain_links_expose_own_metadata() {
    let report = report!("has location");
    let root = report.chain().next().expect("at least one link");

    assert!(root.location().is_some(), "own link carries a location");
    assert!(!root.is_transparent(), "a message report is not transparent");
    assert!(root.meta().is_some(), "own link exposes a ReportMeta view");
}

#[test]
fn chain_marks_a_transparent_link() {
    // report!() is a transparent anchor. Wrap a real error under it so the chain
    // has a transparent link to observe.
    let report = report!().with_report(report!(TestError));
    let has_transparent = report.chain().any(|link| link.is_transparent());
    assert!(has_transparent, "the transparent anchor must be visible in the chain");
}

#[test]
fn chain_descends_through_an_arc_report_link() {
    // An ArcReport link branches through report_branch()'s ArcReport arm; the
    // chain must reach the shared inner report.
    let shared = std::sync::Arc::new(report!(Io("shared cause")));
    let report = report!("top").with_report(report!(shared));

    let reached = report
        .chain()
        .any(|link| link.as_error().to_string() == "io: shared cause");
    assert!(reached, "the Arc-shared inner report is reachable via the chain");
}

#[test]
fn source_walks_every_link_kind() {
    // ReportError::source must expose the inner error for each wrapping
    // ReportType (this is the inherent method the Error impl delegates to).
    let arc = report!(std::sync::Arc::new(report!(TestError)));
    assert!((*arc).source().is_some(), "ArcReport exposes a source");

    let nested = report!(report!(TestError));
    assert!((*nested).source().is_some(), "Report exposes a source");

    let wrapped = report!(TestError).with_message("ctx");
    assert!((*wrapped).source().is_some(), "Wrapper exposes its cause as source");

    // Leaves have no source.
    assert!((*report!("leaf")).source().is_none(), "Message has no source");
    assert!((*report!()).source().is_none(), "Transparent has no source");
}

#[test]
fn chain_descends_into_a_foreign_source() {
    // A std error carrying its own `source()` is a Foreign link once reached;
    // it has no ReportMeta view, so location() / meta() are None but it is
    // still yielded and rendered.
    #[derive(Debug)]
    struct Inner;
    impl std::fmt::Display for Inner {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("inner cause")
        }
    }
    impl std::error::Error for Inner {}

    #[derive(Debug)]
    struct Outer(Inner);
    impl std::fmt::Display for Outer {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("outer")
        }
    }
    impl std::error::Error for Outer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    let report = report!(Outer(Inner));
    let foreign = report
        .chain()
        .find(|link| link.as_error().to_string() == "inner cause")
        .expect("foreign source link is reachable");

    // Foreign links have no ReportMeta view.
    assert!(foreign.meta().is_none());
    assert!(foreign.location().is_none());
    assert!(!foreign.is_transparent(), "foreign errors are never transparent");
}

#[test]
fn chain_shallow_wrapper_yields_correct_count() {
    // report!(Io("loop")).with_message("context")
    // Produces: Wrapper(root) -> [Message, Cause(Io)] -> Io
    // 4 items: wrapper report, message report, cause report, leaf error.
    let report = report!(Io("loop")).with_message("context");
    let links: Vec<_> = report.chain().collect();
    assert_eq!(links.len(), 4, "wrapper + message + cause + error");
    assert!(links.iter().any(|l| l.as_error().to_string() == "context"));
    assert!(links.iter().any(|l| l.as_error().to_string() == "io: loop"));
}

#[test]
fn chain_deep_wrappers_yields_correct_count() {
    // report!(TestError)
    //   .with_message("level 4")
    //   .with_message("level 3")
    //   .with_message("level 2")
    //   .with_message("level 1")
    //
    // Structure: Wrapper(4, Wrapper(3, Wrapper(2, Wrapper(1, Report(Error(TestError))))))
    // Yields in DFS order:
    //   [0] Wrapper(4) report
    //   [1] Message("level 4") report
    //   [2] Wrapper(3) report
    //   [3] Message("level 3") report
    //   [4] Wrapper(2) report
    //   [5] Message("level 2") report
    //   [6] Wrapper(1) report
    //   [7] Message("level 1") report
    //   [8] Error(TestError) report
    //   [9] TestError
    // Total: 4 wrapper + 4 message + 1 error report + 1 leaf error = 10.
    let report = report!(TestError)
        .with_message("level 4")
        .with_message("level 3")
        .with_message("level 2")
        .with_message("level 1");
    let mut chain = report.chain();

    // Structure: level1(wrapper( level2(wrapper( level3(wrapper( level4(wrapper(
    //   Report(Error(TestError)), "level4" )), "level3" )), "level2" )),
    //   "level1" ))
    // chain() yields from innermost (TestError) outward to outermost wrapper.
    assert_eq!(chain.next().unwrap().as_error().to_string(), "level 1");
    assert_eq!(chain.next().unwrap().as_error().to_string(), "level 1");
    assert_eq!(chain.next().unwrap().as_error().to_string(), "level 2");
    assert_eq!(chain.next().unwrap().as_error().to_string(), "level 2");
    assert_eq!(chain.next().unwrap().as_error().to_string(), "level 3");
    assert_eq!(chain.next().unwrap().as_error().to_string(), "level 3");
    assert_eq!(chain.next().unwrap().as_error().to_string(), "level 4");
    assert_eq!(chain.next().unwrap().as_error().to_string(), "level 4");
    assert_eq!(chain.next().unwrap().as_error().to_string(), "TestError");
    assert_eq!(chain.next().unwrap().as_error().to_string(), "TestError");
    assert!(chain.next().is_none());
}

