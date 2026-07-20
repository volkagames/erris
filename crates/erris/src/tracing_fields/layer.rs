use super::*;
use tracing_core::span::{Attributes, Id, Record};
use tracing_core::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

#[derive(Debug, Clone)]
pub struct Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    _s: std::marker::PhantomData<S>,
}

impl<S> Default for Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn default() -> Self {
        Self {
            _s: Default::default(),
        }
    }
}

impl<S> tracing_subscriber::layer::Layer<S> for Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        {
            let span = ctx.span(id).expect("Span not found, this is a bug");
            let mut extensions = span.extensions_mut();

            if extensions.get_mut::<LayerFields>().is_none() {
                let mut fields = LayerFields::new();

                attrs.record(&mut JsonVisitor::new(&mut fields));

                extensions.insert(fields);
            }
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        {
            let span = ctx.span(id).expect("Span not found, this is a bug");
            let mut extensions = span.extensions_mut();
            match extensions.get_mut::<LayerFields>() {
                Some(field_map) => values.record(&mut JsonVisitor::new(field_map)),
                None => {
                    let mut fields = LayerFields::new();

                    values.record(&mut JsonVisitor::new(&mut fields));

                    extensions.insert(fields);
                }
            }
        }
    }
}
