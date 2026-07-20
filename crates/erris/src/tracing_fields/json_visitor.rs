use std::fmt;
use tracing_core::Field;
use tracing_subscriber::field::Visit;

/// A map of fields to values for a given span
pub type LayerFields = serde_json::Map<String, serde_json::Value>;

pub(crate) struct JsonVisitor<'a> {
    values: &'a mut LayerFields,
}

impl<'a> JsonVisitor<'a> {
    /// Returns a new default visitor using the provided writer
    pub(crate) fn new(values: &'a mut LayerFields) -> Self {
        Self { values }
    }
}

impl Visit for JsonVisitor<'_> {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.values
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.values
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.values
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.values
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.values.insert(
            field.name().to_string(),
            serde_json::Value::from(format!("{value:?}")),
        );
    }

    #[cfg(all(tracing_unstable, feature = "valuable"))]
    fn record_value(&mut self, field: &Field, value: valuable::Value<'_>) {
        let value = serde_json::to_value(valuable_serde::Serializable::new(value)).unwrap();

        self.values.insert(field.name().to_string(), value);
    }
}

impl fmt::Debug for JsonVisitor<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .debug_struct("Visitor")
            .field("values", &self.values)
            .finish()
    }
}
