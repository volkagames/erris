mod global_subscriber_registry;
mod json_visitor;
mod layer;

pub use global_subscriber_registry::*;
pub use json_visitor::*;
pub use layer::*;

pub fn current_span_id() -> Option<tracing::Id> {
    tracing::dispatcher::get_default(|dispatcher| dispatcher.current_span().id().cloned())
}
