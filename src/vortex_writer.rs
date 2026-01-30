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
}

impl VortexWriter {
    pub fn new() -> Self {
        Self {
            field_order: Vec::new(),
            records: Vec::new(),
        }
    }

    pub async fn add_record(&mut self, record: Value) -> Result<()> {
        if self.field_order.is_empty() {
            if let Some(obj) = record.as_object() {
                self.field_order = obj.keys().cloned().collect();
            }
        }
        
        self.records.push(record);
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
