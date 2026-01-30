use anyhow::Result;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;

pub struct JsonStreamReader {
    reader: BufReader<ChildStdout>,
    line_buffer: String,
}

impl JsonStreamReader {
    pub fn new(stdout: ChildStdout) -> Self {
        Self {
            reader: BufReader::new(stdout),
            line_buffer: String::new(),
        }
    }

    pub async fn read_all_json(&mut self) -> Result<Vec<Value>> {
        // Read ALL stdout content first
        let mut full_output = String::new();
        
        loop {
            self.line_buffer.clear();
            let bytes_read = self.reader.read_line(&mut self.line_buffer).await?;
            
            if bytes_read == 0 {
                break;
            }
            
            full_output.push_str(&self.line_buffer);
        }

        tracing::info!("Parsing JSON content ({} bytes)", full_output.len());

        // Find the start of JSON - look for {"results" specifically
        let json_start = full_output.find("{\"results\"")
            .or_else(|| full_output.find('['))
            .unwrap_or(0);

        // Remove trailing text after JSON ends
        let mut json_content = &full_output[json_start..];
        
        // Try to find the end of JSON by looking for common trailing patterns
        if let Some(pos) = json_content.find("Déconnecté") {
            json_content = &json_content[..pos];
        } else if let Some(pos) = json_content.find("Version ") {
            json_content = &json_content[..pos];
        } else if let Some(pos) = json_content.find("Oracle ") {
            json_content = &json_content[..pos];
        }

        // Trim whitespace
        json_content = json_content.trim();

        // Parse the JSON structure
        let parsed: Value = serde_json::from_str(json_content)
            .map_err(|e| {
                // If JSON parsing fails, log what we tried to parse for debugging
                tracing::error!("Failed to parse JSON. First 500 chars: {}", 
                    if json_content.len() > 500 {
                        &json_content[..500]
                    } else {
                        json_content
                    }
                );
                tracing::error!("Full output length: {} bytes", full_output.len());
                e
            })?;

        // Extract records from SQLcl's structure: {"results":[{"items":[...]}]}
        let records = if let Some(results) = parsed.get("results").and_then(|v| v.as_array()) {
            if let Some(first_result) = results.first() {
                if let Some(items) = first_result.get("items").and_then(|v| v.as_array()) {
                    items.clone()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else if let Some(array) = parsed.as_array() {
            // Fallback: if it's already an array
            array.clone()
        } else {
            // Fallback: single object
            vec![parsed]
        };

        tracing::info!("Successfully parsed {} records from JSON", records.len());

        Ok(records)
    }
}
