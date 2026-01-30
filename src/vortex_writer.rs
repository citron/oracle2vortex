use anyhow::{Context, Result};
use serde_json::Value;
use std::path::Path;
use vortex_array::arrays::{BoolArray, PrimitiveArray, StructArray, VarBinArray};
use vortex_array::validity::Validity;
use vortex_array::IntoArray;
use vortex_buffer::Buffer;
use vortex_dtype::{DType, Nullability, PType};
use vortex_file::WriteOptionsSessionExt;
use vortex_io::session::RuntimeSession;
use vortex_session::VortexSession;

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
            Value::String(_) => DType::Utf8(Nullability::Nullable),
            _ => DType::Utf8(Nullability::Nullable), // Fallback
        }
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
