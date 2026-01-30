use anyhow::Result;
use serde_json::{Value, Map};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;

// CSV streaming functionality - currently unused but kept for potential future CSV mode
#[allow(dead_code)]
pub struct CsvStreamReader {
    reader: BufReader<ChildStdout>,
    line_buffer: String,
    headers: Vec<String>,
    batch_size: usize,
}

#[allow(dead_code)]
impl CsvStreamReader {
    pub fn new(stdout: ChildStdout, batch_size: usize) -> Self {
        Self {
            reader: BufReader::new(stdout),
            line_buffer: String::new(),
            headers: Vec::new(),
            batch_size,
        }
    }

    /// Read and parse the header line
    async fn read_headers(&mut self) -> Result<()> {
        // Skip any leading lines until we find a line that looks like headers
        loop {
            self.line_buffer.clear();
            let bytes_read = self.reader.read_line(&mut self.line_buffer).await?;
            
            if bytes_read == 0 {
                anyhow::bail!("No headers found in CSV output");
            }

            let line = self.line_buffer.trim();
            
            // Skip empty lines and SQLcl messages
            if line.is_empty() || line.starts_with("SQLcl") || line.starts_with("Copyright") {
                continue;
            }
            
            // Found header line - parse CSV headers
            let mut headers = Vec::new();
            let mut current = String::new();
            let mut in_quotes = false;
            
            for ch in line.chars() {
                match ch {
                    '"' => {
                        in_quotes = !in_quotes;
                    }
                    ',' if !in_quotes => {
                        headers.push(current.trim().trim_matches('"').to_string());
                        current.clear();
                    }
                    _ => {
                        current.push(ch);
                    }
                }
            }
            headers.push(current.trim().trim_matches('"').to_string());
            
            self.headers = headers;
            
            tracing::info!("CSV headers found: {} columns", self.headers.len());
            break;
        }
        
        Ok(())
    }

    /// Parse a CSV line into a JSON object
    fn parse_line(&self, line: &str) -> Option<Value> {
        // Use a simple CSV parser - split by comma but respect quotes
        let mut values = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        
        for ch in line.chars() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                }
                ',' if !in_quotes => {
                    values.push(current.trim().to_string());
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        values.push(current.trim().to_string());

        if values.len() != self.headers.len() {
            tracing::warn!("Column count mismatch: expected {}, got {}", 
                self.headers.len(), values.len());
            return None;
        }

        let mut map = Map::new();
        for (header, value) in self.headers.iter().zip(values.iter()) {
            let json_value = if value.is_empty() || value == "\"\"" {
                Value::Null
            } else if let Ok(num) = value.parse::<i64>() {
                Value::Number(num.into())
            } else if let Ok(num) = value.parse::<f64>() {
                Value::Number(serde_json::Number::from_f64(num).unwrap_or(0.into()))
            } else if value.eq_ignore_ascii_case("true") {
                Value::Bool(true)
            } else if value.eq_ignore_ascii_case("false") {
                Value::Bool(false)
            } else {
                Value::String(value.to_string())
            };
            
            map.insert(header.clone(), json_value);
        }

        Some(Value::Object(map))
    }

    /// Read the next batch of records
    pub async fn read_batch(&mut self) -> Result<Vec<Value>> {
        // Read headers if not already read
        if self.headers.is_empty() {
            self.read_headers().await?;
        }

        let mut batch = Vec::with_capacity(self.batch_size);
        
        loop {
            self.line_buffer.clear();
            let bytes_read = self.reader.read_line(&mut self.line_buffer).await?;
            
            // End of stream
            if bytes_read == 0 {
                break;
            }

            let line = self.line_buffer.trim();
            
            // Skip empty lines and messages
            if line.is_empty() || 
               line.starts_with("SQLcl") || 
               line.starts_with("Copyright") ||
               line.starts_with("Déconnecté") ||
               line.starts_with("Version") ||
               line.contains("ligne") && line.contains("sélectionnée") {
                continue;
            }

            // Try to parse the line
            if let Some(record) = self.parse_line(line) {
                batch.push(record);
                
                // If we've reached batch size, return this batch
                if batch.len() >= self.batch_size {
                    break;
                }
            }
        }

        if !batch.is_empty() {
            tracing::info!("Read batch of {} records", batch.len());
        }

        Ok(batch)
    }

    /// Read all remaining records (for compatibility with existing code)
    pub async fn read_all(&mut self) -> Result<Vec<Value>> {
        let mut all_records = Vec::new();
        
        loop {
            let batch = self.read_batch().await?;
            if batch.is_empty() {
                break;
            }
            all_records.extend(batch);
        }

        Ok(all_records)
    }
}
