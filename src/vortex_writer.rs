use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use vortex_array::arrays::{BoolArray, PrimitiveArray, StructArray, VarBinArray};
use vortex_array::validity::Validity;
use vortex_array::IntoArray;
use vortex_buffer::Buffer;
use vortex_dtype::{DType, ExtDType, Nullability, PType};
use vortex_dtype::datetime::{TemporalMetadata, TimeUnit, DATE_ID, TIMESTAMP_ID};
use vortex_file::WriteOptionsSessionExt;
use vortex_io::session::RuntimeSession;
use vortex_session::VortexSession;
use jiff::civil::{Date, DateTime};

pub struct VortexWriter {
    field_order: Vec<String>,
    records: Vec<Value>,
    skip_lobs: bool,
}

impl VortexWriter {
    pub fn new(skip_lobs: bool) -> Self {
        Self {
            field_order: Vec::new(),
            records: Vec::new(),
            skip_lobs,
        }
    }

    /// Check if a column value appears to be a LOB type based on heuristics
    /// Oracle LOBs in JSON export can be very long strings or have specific patterns
    fn is_likely_lob(value: &Value) -> bool {
        match value {
            Value::String(s) => {
                // LOBs are often very long strings (> 4000 chars is typical indicator)
                // Or they contain binary data indicators
                s.len() > 4000 || s.starts_with("HEXTORAW")
            }
            _ => false,
        }
    }

    /// Filter out LOB columns from a record
    fn filter_lobs(&self, record: &Value) -> Value {
        if !self.skip_lobs {
            return record.clone();
        }

        if let Some(obj) = record.as_object() {
            let filtered: serde_json::Map<String, Value> = obj
                .iter()
                .filter(|(_, v)| !Self::is_likely_lob(v))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Value::Object(filtered)
        } else {
            record.clone()
        }
    }

    pub async fn add_record(&mut self, record: Value) -> Result<()> {
        // Filter LOBs if skip_lobs is enabled
        let filtered_record = self.filter_lobs(&record);
        
        if self.field_order.is_empty() {
            if let Some(obj) = filtered_record.as_object() {
                self.field_order = obj.keys().cloned().collect();
                
                if self.skip_lobs {
                    let original_count = record.as_object().map(|o| o.len()).unwrap_or(0);
                    let filtered_count = obj.len();
                    if original_count > filtered_count {
                        tracing::info!(
                            "Skipping {} LOB columns (keeping {} columns)",
                            original_count - filtered_count,
                            filtered_count
                        );
                    }
                }
            }
        }
        
        self.records.push(filtered_record);
        Ok(())
    }

    fn infer_dtype(value: &Value) -> DType {
        match value {
            Value::Null => DType::Utf8(Nullability::Nullable),
            Value::Bool(_) => DType::Bool(Nullability::Nullable),
            Value::Number(n) => {
                if n.is_f64() {
                    DType::Primitive(PType::F64, Nullability::Nullable)
                } else {
                    DType::Primitive(PType::I64, Nullability::Nullable)
                }
            }
            Value::String(s) => {
                // Detect ISO 8601 date/timestamp patterns
                if Self::is_iso_date(s) {
                    // Pure date: YYYY-MM-DD
                    let metadata = TemporalMetadata::Date(TimeUnit::Days);
                    let ext_dtype = ExtDType::new(
                        DATE_ID.clone(),
                        Arc::new(DType::Primitive(PType::I32, Nullability::Nullable)),
                        Some(metadata.into()),
                    );
                    DType::Extension(Arc::new(ext_dtype))
                } else if Self::is_iso_timestamp(s) {
                    // Timestamp with optional fractional seconds and timezone
                    let metadata = TemporalMetadata::Timestamp(TimeUnit::Microseconds, None);
                    let ext_dtype = ExtDType::new(
                        TIMESTAMP_ID.clone(),
                        Arc::new(DType::Primitive(PType::I64, Nullability::Nullable)),
                        Some(metadata.into()),
                    );
                    DType::Extension(Arc::new(ext_dtype))
                } else {
                    DType::Utf8(Nullability::Nullable)
                }
            }
            _ => DType::Utf8(Nullability::Nullable), // Fallback
        }
    }

    /// Check if a string is an ISO 8601 date (YYYY-MM-DD)
    fn is_iso_date(s: &str) -> bool {
        // Match YYYY-MM-DD format
        s.len() == 10 && 
        s.chars().nth(4) == Some('-') && 
        s.chars().nth(7) == Some('-') &&
        Date::strptime("%Y-%m-%d", s).is_ok()
    }

    /// Check if a string is an ISO 8601 timestamp (YYYY-MM-DDTHH:MM:SS[.ffffff][Z|±HH:MM])
    fn is_iso_timestamp(s: &str) -> bool {
        // Match YYYY-MM-DDTHH:MM:SS format (with optional fractional seconds and timezone)
        s.contains('T') && DateTime::strptime("%Y-%m-%dT%H:%M:%S", &s[..19]).is_ok()
    }

    /// Parse ISO 8601 date to days since epoch
    fn parse_date_to_days(s: &str) -> Option<i32> {
        let date = Date::strptime("%Y-%m-%d", s).ok()?;
        let epoch = Date::new(1970, 1, 1).ok()?;
        date.since(epoch).ok()?.get_days().try_into().ok()
    }

    /// Parse ISO 8601 timestamp to microseconds since epoch
    fn parse_timestamp_to_micros(s: &str) -> Option<i64> {
        // Try parsing with fractional seconds first
        if s.len() < 19 {
            return None;
        }
        
        // Handle YYYY-MM-DDTHH:MM:SS.ffffff format
        let base_format = "%Y-%m-%dT%H:%M:%S";
        let base_part = &s[..19];
        let dt = DateTime::strptime(base_format, base_part).ok()?;
        
        // Extract fractional seconds if present
        let micros_fraction = if s.len() > 19 && s.chars().nth(19) == Some('.') {
            let frac_part = s[20..].split(|c: char| !c.is_ascii_digit()).next().unwrap_or("0");
            let frac_str = format!("{:0<6}", &frac_part[..frac_part.len().min(6)]);
            frac_str.parse::<i64>().unwrap_or(0)
        } else {
            0
        };
        
        // Convert to microseconds since epoch
        let epoch = DateTime::new(1970, 1, 1, 0, 0, 0, 0).ok()?;
        let duration = dt.since(epoch).ok()?;
        
        // Calculate total microseconds: (seconds * 1_000_000) + microseconds_fraction
        let total_micros = duration.get_seconds() * 1_000_000 + micros_fraction;
        
        Some(total_micros)
    }

    pub async fn flush<P: AsRef<Path>>(&mut self, output_path: P) -> Result<()> {
        if self.records.is_empty() {
            tracing::warn!("No records to write");
            return Ok(());
        }

        tracing::info!("Writing {} records to Vortex file", self.records.len());

        // Determine field order from first record if not set
        if self.field_order.is_empty() {
            if let Some(obj) = self.records[0].as_object() {
                self.field_order = obj.keys().cloned().collect();
            }
        }

        // Build column arrays
        let mut fields = Vec::new();

        for field_name in &self.field_order {
            // Infer dtype from first non-null value
            let dtype = self.records.iter()
                .find_map(|r| r.as_object()?.get(field_name))
                .map(|v| Self::infer_dtype(v))
                .unwrap_or(DType::Utf8(Nullability::Nullable));

            tracing::debug!("Field '{}': dtype={:?}, len={}", field_name, dtype, self.records.len());

            let array = match dtype {
                DType::Primitive(PType::I64, _) => {
                    let mut values = Vec::with_capacity(self.records.len());
                    let mut validity = Vec::with_capacity(self.records.len());

                    for record in &self.records {
                        if let Some(obj) = record.as_object() {
                            if let Some(val) = obj.get(field_name) {
                                match val {
                                    Value::Number(n) => {
                                        values.push(n.as_i64().unwrap_or(0));
                                        validity.push(true);
                                    }
                                    Value::Null => {
                                        values.push(0);
                                        validity.push(false);
                                    }
                                    _ => {
                                        values.push(0);
                                        validity.push(false);
                                    }
                                }
                            } else {
                                values.push(0);
                                validity.push(false);
                            }
                        }
                    }

                    let buffer = Buffer::from(values);
                    let validity: Validity = validity.into_iter().collect();
                    PrimitiveArray::new(buffer, validity).into_array()
                }
                DType::Primitive(PType::F64, _) => {
                    let mut values = Vec::with_capacity(self.records.len());
                    let mut validity = Vec::with_capacity(self.records.len());

                    for record in &self.records {
                        if let Some(obj) = record.as_object() {
                            if let Some(val) = obj.get(field_name) {
                                match val {
                                    Value::Number(n) => {
                                        values.push(n.as_f64().unwrap_or(0.0));
                                        validity.push(true);
                                    }
                                    Value::Null => {
                                        values.push(0.0);
                                        validity.push(false);
                                    }
                                    _ => {
                                        values.push(0.0);
                                        validity.push(false);
                                    }
                                }
                            } else {
                                values.push(0.0);
                                validity.push(false);
                            }
                        }
                    }

                    let buffer = Buffer::from(values);
                    let validity: Validity = validity.into_iter().collect();
                    PrimitiveArray::new(buffer, validity).into_array()
                }
                DType::Utf8(_) => {
                    let values: Vec<Option<String>> = self.records.iter()
                        .map(|record| {
                            record.as_object()
                                .and_then(|obj| obj.get(field_name))
                                .and_then(|val| match val {
                                    Value::String(s) => Some(s.clone()),
                                    Value::Null => None,
                                    _ => Some(val.to_string()),
                                })
                        })
                        .collect();

                    VarBinArray::from(values).into_array()
                }
                DType::Bool(_) => {
                    let mut values = Vec::with_capacity(self.records.len());
                    let mut validity = Vec::with_capacity(self.records.len());

                    for record in &self.records {
                        if let Some(obj) = record.as_object() {
                            if let Some(val) = obj.get(field_name) {
                                match val {
                                    Value::Bool(b) => {
                                        values.push(*b);
                                        validity.push(true);
                                    }
                                    Value::Null => {
                                        values.push(false);
                                        validity.push(false);
                                    }
                                    _ => {
                                        values.push(false);
                                        validity.push(false);
                                    }
                                }
                            } else {
                                values.push(false);
                                validity.push(false);
                            }
                        }
                    }

                    let validity: Validity = validity.into_iter().collect();
                    let bits: vortex_buffer::BitBuffer = values.into();
                    BoolArray::new(bits, validity).into_array()
                }
                DType::Extension(ref ext) if ext.id() == &*DATE_ID => {
                    // Handle Date type (days since epoch as I32)
                    let mut values = Vec::with_capacity(self.records.len());
                    let mut validity = Vec::with_capacity(self.records.len());

                    for record in &self.records {
                        if let Some(obj) = record.as_object() {
                            if let Some(val) = obj.get(field_name) {
                                match val {
                                    Value::String(s) => {
                                        if let Some(days) = Self::parse_date_to_days(s) {
                                            values.push(days);
                                            validity.push(true);
                                        } else {
                                            values.push(0);
                                            validity.push(false);
                                        }
                                    }
                                    Value::Null => {
                                        values.push(0);
                                        validity.push(false);
                                    }
                                    _ => {
                                        values.push(0);
                                        validity.push(false);
                                    }
                                }
                            } else {
                                values.push(0);
                                validity.push(false);
                            }
                        }
                    }

                    let buffer = Buffer::from(values);
                    let validity: Validity = validity.into_iter().collect();
                    PrimitiveArray::new(buffer, validity).into_array()
                }
                DType::Extension(ref ext) if ext.id() == &*TIMESTAMP_ID => {
                    // Handle Timestamp type (microseconds since epoch as I64)
                    let mut values = Vec::with_capacity(self.records.len());
                    let mut validity = Vec::with_capacity(self.records.len());

                    for record in &self.records {
                        if let Some(obj) = record.as_object() {
                            if let Some(val) = obj.get(field_name) {
                                match val {
                                    Value::String(s) => {
                                        if let Some(micros) = Self::parse_timestamp_to_micros(s) {
                                            values.push(micros);
                                            validity.push(true);
                                        } else {
                                            values.push(0);
                                            validity.push(false);
                                        }
                                    }
                                    Value::Null => {
                                        values.push(0);
                                        validity.push(false);
                                    }
                                    _ => {
                                        values.push(0);
                                        validity.push(false);
                                    }
                                }
                            } else {
                                values.push(0);
                                validity.push(false);
                            }
                        }
                    }

                    let buffer = Buffer::from(values);
                    let validity: Validity = validity.into_iter().collect();
                    PrimitiveArray::new(buffer, validity).into_array()
                }
                _ => {
                    // Fallback: convert to strings
                    let values: Vec<Option<String>> = self.records.iter()
                        .map(|record| {
                            record.as_object()
                                .and_then(|obj| obj.get(field_name))
                                .and_then(|val| {
                                    if val.is_null() {
                                        None
                                    } else {
                                        Some(val.to_string())
                                    }
                                })
                        })
                        .collect();

                    VarBinArray::from(values).into_array()
                }
            };

            fields.push((field_name.as_str(), array));
        }

        // Create StructArray
        let struct_array = StructArray::from_fields(&fields)
            .context("Failed to create StructArray")?;

        tracing::info!("StructArray created with {} fields and {} rows", fields.len(), struct_array.len());

        // Write to buffer first using session with runtime
        use vortex_io::session::RuntimeSessionExt;
        let session = VortexSession::empty()
            .with::<RuntimeSession>()
            .with_tokio();
        let mut buf = Vec::new();
        
        session
            .write_options()
            .write(&mut buf, struct_array.to_array_stream())
            .await
            .context("Failed to write Vortex file to buffer")?;

        // Write buffer to file with tokio
        tokio::fs::write(output_path.as_ref(), buf).await
            .context("Failed to write buffer to file")?;

        tracing::info!("Successfully wrote {} records to Vortex file", self.records.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vortex_dtype::datetime::{DATE_ID, TIMESTAMP_ID};

    #[test]
    fn test_is_iso_date() {
        assert!(VortexWriter::is_iso_date("2024-03-15"));
        assert!(VortexWriter::is_iso_date("1970-01-01"));
        assert!(!VortexWriter::is_iso_date("2024-03-15T14:30:45")); // This is a timestamp
        assert!(!VortexWriter::is_iso_date("2024/03/15")); // Wrong format
        assert!(!VortexWriter::is_iso_date("not-a-date"));
    }

    #[test]
    fn test_is_iso_timestamp() {
        assert!(VortexWriter::is_iso_timestamp("2024-03-15T14:30:45"));
        assert!(VortexWriter::is_iso_timestamp("2024-03-15T14:30:45.123456"));
        assert!(!VortexWriter::is_iso_timestamp("2024-03-15")); // This is just a date
        assert!(!VortexWriter::is_iso_timestamp("2024-03-15 14:30:45")); // Space instead of T
        assert!(!VortexWriter::is_iso_timestamp("not-a-timestamp"));
    }

    #[test]
    fn test_parse_date_to_days() {
        // 1970-01-01 = epoch = 0 days
        assert_eq!(VortexWriter::parse_date_to_days("1970-01-01"), Some(0));
        
        // 1970-01-02 = 1 day after epoch
        assert_eq!(VortexWriter::parse_date_to_days("1970-01-02"), Some(1));
        
        // 2024-01-01 is 19723 days after epoch
        assert_eq!(VortexWriter::parse_date_to_days("2024-01-01"), Some(19723));
        
        // Invalid date
        assert_eq!(VortexWriter::parse_date_to_days("invalid"), None);
    }

    #[test]
    fn test_parse_timestamp_to_micros() {
        // 1970-01-01T00:00:00 = epoch = 0 microseconds
        assert_eq!(VortexWriter::parse_timestamp_to_micros("1970-01-01T00:00:00"), Some(0));
        
        // 1970-01-01T00:00:01 = 1 second = 1,000,000 microseconds
        assert_eq!(VortexWriter::parse_timestamp_to_micros("1970-01-01T00:00:01"), Some(1_000_000));
        
        // With fractional seconds
        assert_eq!(
            VortexWriter::parse_timestamp_to_micros("1970-01-01T00:00:00.123456"),
            Some(123_456)
        );
        
        // Invalid timestamp
        assert_eq!(VortexWriter::parse_timestamp_to_micros("invalid"), None);
        assert_eq!(VortexWriter::parse_timestamp_to_micros("2024-03-15"), None); // Too short
    }

    #[test]
    fn test_infer_dtype_date() {
        let value = serde_json::json!("2024-03-15");
        let dtype = VortexWriter::infer_dtype(&value);
        
        if let DType::Extension(ext) = &dtype {
            assert_eq!(ext.id(), &*DATE_ID);
        } else {
            panic!("Expected Extension(DATE) type, got {:?}", dtype);
        }
    }

    #[test]
    fn test_infer_dtype_timestamp() {
        let value = serde_json::json!("2024-03-15T14:30:45");
        let dtype = VortexWriter::infer_dtype(&value);
        
        if let DType::Extension(ext) = &dtype {
            assert_eq!(ext.id(), &*TIMESTAMP_ID);
        } else {
            panic!("Expected Extension(TIMESTAMP) type, got {:?}", dtype);
        }
    }

    #[test]
    fn test_infer_dtype_string() {
        let value = serde_json::json!("just a string");
        let dtype = VortexWriter::infer_dtype(&value);
        
        assert!(matches!(dtype, DType::Utf8(_)));
    }

    #[test]
    fn test_infer_dtype_number() {
        let value = serde_json::json!(42);
        let dtype = VortexWriter::infer_dtype(&value);
        
        assert!(matches!(dtype, DType::Primitive(PType::I64, _)));
    }

    #[test]
    fn test_infer_dtype_float() {
        let value = serde_json::json!(42.5);
        let dtype = VortexWriter::infer_dtype(&value);
        
        assert!(matches!(dtype, DType::Primitive(PType::F64, _)));
    }
}
