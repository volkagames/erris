use std::fmt;
use tracing_core::Field;
use tracing_subscriber::field::Visit;

/// A map of fields to values for a given span
pub type LayerFields = serde_json::Map<String, serde_json::Value>;

pub struct JsonVisitor<'a> {
    values: &'a mut LayerFields,
}

impl<'a> JsonVisitor<'a> {
    /// Returns a new default visitor using the provided writer
    pub fn new(values: &'a mut LayerFields) -> Self {
        Self { values }
    }

    /// Record an arbitrary `serde_json::Value` into the layer fields.
    ///
    /// This method is not called by `tracing_core`; it exists for users
    /// who want to inject pre-serialised values (e.g. from `serde_json`)
    /// into the JSON span fields.
    ///
    /// Enable via the `serde_json_value` feature:
    /// ```toml
    /// [dependencies]
    /// erris = { version = "2", features = ["serde_json_value"] }
    /// ```
    #[cfg(feature = "serde_json_value")]
    pub fn record_json(&mut self, field: &Field, value: impl Into<serde_json::Value>) {
        self.values
            .insert(field.name().to_string(), value.into());
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
}

impl fmt::Debug for JsonVisitor<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .debug_struct("Visitor")
            .field("values", &self.values)
            .finish()
    }
}
