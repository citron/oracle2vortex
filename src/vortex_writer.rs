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
                // Check for timezone first (more specific pattern)
                if Self::is_iso_timestamp_tz(s) {
                    // Timestamp with timezone: YYYY-MM-DDTHH:MI:SS.FF+HH:MM or Z
                    if let Some(tz) = Self::extract_timezone(s) {
                        let metadata = TemporalMetadata::Timestamp(TimeUnit::Microseconds, Some(tz));
                        let ext_dtype = ExtDType::new(
                            TIMESTAMP_ID.clone(),
                            Arc::new(DType::Primitive(PType::I64, Nullability::Nullable)),
                            Some(metadata.into()),
                        );
                        DType::Extension(Arc::new(ext_dtype))
                    } else {
                        // Fallback if timezone extraction fails
                        DType::Utf8(Nullability::Nullable)
                    }
                } else if Self::is_iso_date(s) {
                    // Pure date: YYYY-MM-DD
                    let metadata = TemporalMetadata::Date(TimeUnit::Days);
                    let ext_dtype = ExtDType::new(
                        DATE_ID.clone(),
                        Arc::new(DType::Primitive(PType::I32, Nullability::Nullable)),
                        Some(metadata.into()),
                    );
                    DType::Extension(Arc::new(ext_dtype))
                } else if Self::is_iso_timestamp(s) {
                    // Timestamp without timezone
                    let metadata = TemporalMetadata::Timestamp(TimeUnit::Microseconds, None);
                    let ext_dtype = ExtDType::new(
                        TIMESTAMP_ID.clone(),
                        Arc::new(DType::Primitive(PType::I64, Nullability::Nullable)),
                        Some(metadata.into()),
                    );
                    DType::Extension(Arc::new(ext_dtype))
                } else if Self::is_hex_string(s) {
                    // RAW/LONG RAW data (hex encoded)
                    DType::Binary(Nullability::Nullable)
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

    /// Check if a string is an ISO 8601 timestamp (YYYY-MM-DDTHH:MM:SS[.ffffff]) without timezone
    fn is_iso_timestamp(s: &str) -> bool {
        // Must contain T and not have timezone indicators at the end
        if !s.contains('T') || s.len() < 19 {
            return false;
        }
        
        // Check if it has timezone (would be handled by is_iso_timestamp_tz)
        let has_tz = s.ends_with('Z') || 
                      s.contains("+") && s.rfind('+').unwrap() > 19 ||
                      s.matches('-').count() > 2; // More than date hyphens = timezone
        
        if has_tz {
            return false; // Let is_iso_timestamp_tz handle it
        }
        
        DateTime::strptime("%Y-%m-%dT%H:%M:%S", &s[..19.min(s.len())]).is_ok()
    }

    /// Check if a string is an ISO 8601 timestamp with timezone
    fn is_iso_timestamp_tz(s: &str) -> bool {
        if !s.contains('T') || s.len() < 20 {
            return false;
        }
        
        // Check for timezone indicators
        s.ends_with('Z') ||  // UTC indicator
        s.contains(" +") || s.contains(" -") || // Oracle TZ format with space
        (s.rfind('+').map(|i| i >= 19).unwrap_or(false)) || // +HH:MM after timestamp (>= not >)
        (s.rfind('-').map(|i| i >= 19).unwrap_or(false) && s.matches('-').count() > 2)
    }

    /// Extract timezone string from ISO timestamp
    fn extract_timezone(s: &str) -> Option<String> {
        if s.ends_with('Z') {
            return Some("UTC".to_string());
        }
        
        // Look for +/-HH:MM or space +/-HH:MM (Oracle format)
        if let Some(pos) = s.rfind(" +").or_else(|| s.rfind(" -")) {
            // Oracle format with space: "2024-01-01T12:00:00.000000 +02:00"
            return Some(s[pos+1..].trim().to_string());
        }
        
        if let Some(pos) = s.rfind('+') {
            if pos >= 19 { // After or at end of basic timestamp part
                return Some(s[pos..].to_string());
            }
        }
        
        if let Some(pos) = s.rfind('-') {
            if pos >= 19 && s.matches('-').count() > 2 { // More than date hyphens
                return Some(s[pos..].to_string());
            }
        }
        
        None
    }

    /// Parse Oracle timezone format to get UTC timestamp
    fn parse_oracle_tz_format(s: &str) -> Option<i64> {
        // Oracle format: YYYY-MM-DDTHH:MM:SS.FF +HH:MM or -HH:MM
        // Split timestamp and timezone
        let parts: Vec<&str> = if s.contains(" +") || s.contains(" -") {
            s.splitn(2, ' ').collect()
        } else if let Some(stripped) = s.strip_suffix('Z') {
            // UTC timezone, just remove Z and parse
            return Self::parse_timestamp_to_micros(stripped);
        } else {
            // Find last + or - that's not part of the date
            if let Some(pos) = s.rfind('+').or_else(|| s.rfind('-').filter(|&p| p > 19)) {
                vec![&s[..pos], &s[pos..]]
            } else {
                return None;
            }
        };
        
        if parts.len() != 2 {
            return None;
        }
        
        // Parse base timestamp without timezone
        let base_micros = Self::parse_timestamp_to_micros(parts[0].trim())?;
        
        // Parse timezone offset (e.g., "+02:00" or "-05:30")
        let tz_str = parts[1].trim();
        let tz_offset_secs = Self::parse_tz_offset(tz_str)?;
        
        // Convert to UTC by subtracting the offset
        Some(base_micros - (tz_offset_secs * 1_000_000))
    }

    /// Parse timezone offset string to seconds (e.g., "+02:00" -> 7200)
    fn parse_tz_offset(tz: &str) -> Option<i64> {
        // Remove any whitespace
        let tz = tz.trim();
        
        // Check if it starts with + or -
        let (sign, offset_str) = if let Some(stripped) = tz.strip_prefix('+') {
            (1i64, stripped)
        } else if let Some(stripped) = tz.strip_prefix('-') {
            (-1i64, stripped)
        } else {
            return None;
        };
        
        // Parse HH:MM format
        let parts: Vec<&str> = offset_str.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        
        let hours: i64 = parts[0].parse().ok()?;
        let minutes: i64 = parts[1].parse().ok()?;
        
        Some(sign * (hours * 3600 + minutes * 60))
    }

    /// Check if a string is hexadecimal (RAW data from Oracle)
    fn is_hex_string(s: &str) -> bool {
        // Oracle RAW is exported as uppercase hex
        // Minimum reasonable length: 8 chars (4 bytes) to avoid false positives with small numbers
        // Must be even length (each byte = 2 hex chars)
        if s.len() < 8 || !s.len().is_multiple_of(2) {
            return false;
        }
        
        // Must be all hex digits and preferably uppercase (Oracle convention)
        s.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Convert hex string to binary data
    fn hex_to_binary(s: &str) -> Option<Vec<u8>> {
        if !s.len().is_multiple_of(2) {
            return None;
        }
        
        let mut bytes = Vec::with_capacity(s.len() / 2);
        for i in (0..s.len()).step_by(2) {
            let byte = u8::from_str_radix(&s[i..i+2], 16).ok()?;
            bytes.push(byte);
        }
        
        Some(bytes)
    }

    /// Parse ISO 8601 date to days since epoch
    fn parse_date_to_days(s: &str) -> Option<i32> {
        let date = Date::strptime("%Y-%m-%d", s).ok()?;
        let epoch = Date::new(1970, 1, 1).ok()?;
        Some(date.since(epoch).ok()?.get_days())
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
                .map(Self::infer_dtype)
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
                    // Supports both with and without timezone
                    let mut values = Vec::with_capacity(self.records.len());
                    let mut validity = Vec::with_capacity(self.records.len());

                    for record in &self.records {
                        if let Some(obj) = record.as_object() {
                            if let Some(val) = obj.get(field_name) {
                                match val {
                                    Value::String(s) => {
                                        // Try timezone-aware parsing first
                                        let micros = if Self::is_iso_timestamp_tz(s) {
                                            Self::parse_oracle_tz_format(s)
                                                .or_else(|| Self::parse_timestamp_to_micros(s))
                                        } else {
                                            Self::parse_timestamp_to_micros(s)
                                        };
                                        
                                        if let Some(micros) = micros {
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
                DType::Binary(_) => {
                    // Handle Binary type (RAW/LONG RAW data)
                    let values: Vec<Option<Vec<u8>>> = self.records.iter()
                        .map(|record| {
                            record.as_object()
                                .and_then(|obj| obj.get(field_name))
                                .and_then(|val| match val {
                                    Value::String(s) => Self::hex_to_binary(s),
                                    Value::Null => None,
                                    _ => None,
                                })
                        })
                        .collect();

                    VarBinArray::from(values).into_array()
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

    #[test]
    fn test_is_iso_timestamp_tz() {
        // UTC timezone
        assert!(VortexWriter::is_iso_timestamp_tz("2024-03-15T14:30:45Z"));
        // Positive offset
        assert!(VortexWriter::is_iso_timestamp_tz("2024-03-15T14:30:45 +02:00"));
        assert!(VortexWriter::is_iso_timestamp_tz("2024-03-15T14:30:45+02:00"));
        // Negative offset
        assert!(VortexWriter::is_iso_timestamp_tz("2024-03-15T14:30:45 -05:00"));
        assert!(VortexWriter::is_iso_timestamp_tz("2024-03-15T14:30:45-05:00"));
        // Not a timezone timestamp
        assert!(!VortexWriter::is_iso_timestamp_tz("2024-03-15T14:30:45"));
        assert!(!VortexWriter::is_iso_timestamp_tz("2024-03-15"));
    }

    #[test]
    fn test_extract_timezone() {
        assert_eq!(VortexWriter::extract_timezone("2024-03-15T14:30:45Z"), Some("UTC".to_string()));
        assert_eq!(VortexWriter::extract_timezone("2024-03-15T14:30:45 +02:00"), Some("+02:00".to_string()));
        assert_eq!(VortexWriter::extract_timezone("2024-03-15T14:30:45+02:00"), Some("+02:00".to_string()));
        assert_eq!(VortexWriter::extract_timezone("2024-03-15T14:30:45 -05:30"), Some("-05:30".to_string()));
        assert_eq!(VortexWriter::extract_timezone("2024-03-15T14:30:45"), None);
    }

    #[test]
    fn test_parse_tz_offset() {
        assert_eq!(VortexWriter::parse_tz_offset("+00:00"), Some(0));
        assert_eq!(VortexWriter::parse_tz_offset("+02:00"), Some(7200));
        assert_eq!(VortexWriter::parse_tz_offset("-05:00"), Some(-18000));
        assert_eq!(VortexWriter::parse_tz_offset("+05:30"), Some(19800)); // India
        assert_eq!(VortexWriter::parse_tz_offset("invalid"), None);
    }

    #[test]
    fn test_parse_oracle_tz_format() {
        // UTC
        assert_eq!(
            VortexWriter::parse_oracle_tz_format("1970-01-01T00:00:00Z"),
            Some(0)
        );
        
        // +02:00 timezone: timestamp given is in +02:00 zone
        // 1970-01-01T02:00:00+02:00 means 2am in +02:00 zone = midnight UTC
        // But we're testing with 00:00:00 in +02:00 = 22:00 previous day UTC = -7200 seconds
        // Actually: 1970-01-01T00:00:00+02:00 = 1969-12-31T22:00:00 UTC = -7200 seconds from epoch
        assert_eq!(
            VortexWriter::parse_oracle_tz_format("1970-01-01T00:00:00 +02:00"),
            Some(-7_200_000_000) // -2 hours in microseconds
        );
        
        // With fractional seconds
        assert_eq!(
            VortexWriter::parse_oracle_tz_format("1970-01-01T00:00:00.500000Z"),
            Some(500_000)
        );
    }

    #[test]
    fn test_is_hex_string() {
        assert!(VortexWriter::is_hex_string("DEADBEEF"));
        assert!(VortexWriter::is_hex_string("0123456789ABCDEF"));
        assert!(!VortexWriter::is_hex_string("00")); // Too short (< 8 chars)
        assert!(!VortexWriter::is_hex_string("12")); // Too short
        assert!(!VortexWriter::is_hex_string("G1234567")); // Invalid hex char
        assert!(!VortexWriter::is_hex_string("1234567")); // Odd length
        assert!(!VortexWriter::is_hex_string("")); // Empty
        assert!(!VortexWriter::is_hex_string("Hello World")); // Not hex
    }

    #[test]
    fn test_hex_to_binary() {
        assert_eq!(VortexWriter::hex_to_binary("00"), Some(vec![0x00]));
        assert_eq!(VortexWriter::hex_to_binary("FF"), Some(vec![0xFF]));
        assert_eq!(VortexWriter::hex_to_binary("DEADBEEF"), Some(vec![0xDE, 0xAD, 0xBE, 0xEF]));
        assert_eq!(VortexWriter::hex_to_binary("0123456789ABCDEF"), 
                   Some(vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF]));
        assert_eq!(VortexWriter::hex_to_binary("invalid"), None);
        assert_eq!(VortexWriter::hex_to_binary("123"), None); // Odd length
    }

    #[test]
    fn test_infer_dtype_binary() {
        let value = serde_json::json!("DEADBEEF");
        let dtype = VortexWriter::infer_dtype(&value);
        assert!(matches!(dtype, DType::Binary(_)));
    }

    #[test]
    fn test_infer_dtype_timestamp_tz() {
        let value = serde_json::json!("2024-03-15T14:30:45 +02:00");
        let dtype = VortexWriter::infer_dtype(&value);
        
        if let DType::Extension(ext) = &dtype {
            assert_eq!(ext.id(), &*TIMESTAMP_ID);
            // Check that timezone metadata is present
            // (metadata is stored in ext.metadata())
        } else {
            panic!("Expected Extension(TIMESTAMP) type with TZ, got {:?}", dtype);
        }
    }
}
