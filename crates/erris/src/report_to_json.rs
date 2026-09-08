use crate::{
    Report,
    collect_causes_without_consecutive_duplicates,
    collect_locations_without_consecutive_duplicates,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonError {
    pub message: String,
    pub cause: Vec<String>,
    pub location: Vec<String>,
    #[cfg(feature = "spantrace")]
    pub spantrace: Vec<JsonErrorSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonErrorSpan {
    pub target: String,
    pub name: String,
    pub location: Option<String>,
    pub field: Option<serde_json::Map<String, serde_json::Value>>,
}

impl From<JsonError> for serde_json::Value {
    fn from(value: JsonError) -> Self {
        let mut result = serde_json::Map::with_capacity(4);
        result.insert("message".to_string(), value.message.into());
        result.insert("cause".to_string(), value.cause.into());
        result.insert("location".to_string(), value.location.into());
        #[cfg(feature = "spantrace")]
        result.insert("spantrace".to_string(), value.spantrace.into());
        result.into()
    }
}

impl From<JsonErrorSpan> for serde_json::Value {
    fn from(value: JsonErrorSpan) -> Self {
        let mut result = serde_json::Map::with_capacity(4);
        result.insert("target".to_string(), value.target.into());
        result.insert("name".to_string(), value.name.into());
        result.insert("location".to_string(), value.location.into());
        if let Some(field) = value.field {
            result.insert("field".to_string(), field.into());
        }
        result.into()
    }
}

impl Report {
    pub fn to_json_error(&self) -> JsonError {
        let error = &*self.boxed;
        JsonError {
            message: self.to_string(),
            cause: collect_causes_without_consecutive_duplicates(error),
            location: collect_locations_without_consecutive_duplicates(error)
                .into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>(),
            #[cfg(feature = "spantrace")]
            spantrace: self
                .span_id()
                .and_then(spantrace::to_json_value_with_span)
                .unwrap_or_else(|| spantrace::to_json_value(self.spantrace())),
        }
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        self.to_json_error().into()
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.to_json_value())
    }

    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&self.to_json_value())
    }

    pub fn to_json_safe(&self) -> String {
        serde_json::to_string(&self.to_json_value()).unwrap_or_else(|_| self.to_string())
    }
}

#[cfg(feature = "spantrace")]
mod spantrace {
    use super::JsonErrorSpan;
    use crate::tracing_fields::get_global_subscriber;
    use tracing::Metadata;
    use tracing::span::Id as SpanId;
    use tracing_subscriber::Registry;
    use tracing_subscriber::registry::{LookupSpan, SpanData};

    pub(crate) fn to_json_value(spantrace: &tracing_error::SpanTrace) -> Vec<JsonErrorSpan> {
        let mut result = vec![];
        spantrace.with_spans(|meta, _| {
            result.push(JsonErrorSpan {
                target: meta.target().to_string(),
                name: meta.name().to_string(),
                location: get_file_location(meta),
                field: None,
            });
            true
        });
        result
    }

    pub(crate) fn to_json_value_with_span(span_id: SpanId) -> Option<Vec<JsonErrorSpan>> {
        let subscriber = get_global_subscriber()?;
        let registry = subscriber.downcast_ref::<Registry>()?;
        Some(span_to_json_recurse(registry, span_id, vec![]))
    }

    fn span_to_json_recurse(
        registry: &Registry,
        span_id: SpanId,
        mut spantrace: Vec<JsonErrorSpan>,
    ) -> Vec<JsonErrorSpan> {
        let Some(data) = registry.span_data(&span_id) else {
            return spantrace;
        };

        let extensions = data.extensions();
        let meta = data.metadata();

        spantrace.push(JsonErrorSpan {
            target: meta.target().to_string(),
            name: meta.name().to_string(),
            location: get_file_location(meta),
            field: extensions
                .get::<serde_json::Map<String, serde_json::Value>>()
                .cloned(),
        });

        if let Some(parent) = data.parent() {
            return span_to_json_recurse(registry, parent.clone(), spantrace);
        }

        spantrace
    }

    fn get_file_location(meta: &Metadata<'_>) -> Option<String> {
        meta.file().map(|file| {
            if let Some(line) = meta.line() {
                format!("{file}:{line}")
            } else {
                file.to_string()
            }
        })
    }
}
