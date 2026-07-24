/// A single link in a report chain.
///
/// `Own` carries the shared [`erris_meta::ReportMeta`] view, so metadata
/// (location, spantrace, transparency) is reachable even when the link comes
/// from a different `erris` version in the same binary. `Foreign` is a
/// third-party error already erased to `dyn Error`; on stable its metadata is
/// unreachable (there is no `dyn Error -> dyn ReportMeta` cast).
#[derive(Debug)]
pub enum ReportLink<'a> {
    Own(&'a dyn erris_meta::ReportMeta),
    Foreign(&'a (dyn std::error::Error + 'static)),
}

impl<'a> ReportLink<'a> {
    /// The link as a plain `&dyn Error` (for Display / source traversal).
    pub fn as_error(&self) -> &'a (dyn std::error::Error + 'static) {
        match self {
            // Upcast dyn ReportMeta -> dyn Error (supertrait, stable since 1.86).
            ReportLink::Own(meta) => *meta as &dyn std::error::Error,
            ReportLink::Foreign(err) => *err,
        }
    }

    /// The `ReportMeta` view, if this is an own link.
    pub fn meta(&self) -> Option<&'a dyn erris_meta::ReportMeta> {
        match self {
            ReportLink::Own(meta) => Some(*meta),
            ReportLink::Foreign(_) => None,
        }
    }

    /// Location captured for this link, if it is an own report link.
    pub fn location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.meta().and_then(|m| m.report_location())
    }

    /// Whether this link is a transparent (message-less) wrapper. Foreign
    /// errors are never transparent in erris's sense.
    pub fn is_transparent(&self) -> bool {
        self.meta()
            .map(|m| m.report_is_transparent())
            .unwrap_or(false)
    }
}

/// Iterates all links in a report chain from innermost to outermost using
/// stack-based DFS. Own links are yielded before their children; foreign
/// links are followed via `Error::source()`.
///
/// # Contract
///
/// Assumes the `Error::source()` chain is acyclic. A cyclic `source()` (which
/// can only be constructed via `unsafe` Rust — safe code cannot create a
/// reference cycle of owned values with `&'static` lifetimes) will cause
/// `next()` to loop forever. No cycle detection is present; this matches
/// `anyhow` and `eyre` which share the same contract.
///
/// Legitimate same-pointer references (e.g. `&Outer` and `&Outer.0` where
/// both point to the same heap allocation) are **not** treated as cycles —
/// each is yielded as a distinct link. This is correct: they represent
/// different views of the same report, not a loop.
#[derive(Debug)]
pub struct ReportIterator<'a> {
    stack: Vec<ReportLink<'a>>,
}

impl<'a> ReportIterator<'a> {
    pub fn new(root: ReportLink<'a>) -> Self {
        Self { stack: vec![root] }
    }
}

impl<'a> Iterator for ReportIterator<'a> {
    type Item = ReportLink<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.stack.pop()?;

        let (lhs, rhs) = branch(&current);
        if let Some(lhs) = lhs {
            self.stack.push(lhs);
        }
        if let Some(rhs) = rhs {
            self.stack.push(rhs);
        }

        Some(current)
    }
}

pub(crate) fn branch<'a>(
    link: &ReportLink<'a>,
) -> (Option<ReportLink<'a>>, Option<ReportLink<'a>>) {
    // Own links branch through the shared trait, so both children of a wrapper
    // (cause and message) survive a version boundary. Foreign errors have no
    // ReportMeta view and fall back to std source() traversal.
    if let Some(meta) = link.meta() {
        let (lhs, rhs) = meta.report_branch();
        let mut children = (lhs.map(ReportLink::Own), rhs.map(ReportLink::Own));
        // A foreign inner error (e.g. ReportType::Error) is not exposed by
        // report_branch(); recover it via source() so the chain continues.
        if children.0.is_none()
            && let Some(source) = link.as_error().source()
        {
            children.0 = Some(ReportLink::Foreign(source));
        }
        return children;
    }
    if let Some(source) = link.as_error().source() {
        return (Some(ReportLink::Foreign(source)), None);
    }
    (None, None)
}
