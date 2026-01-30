use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

use crate::json_stream::JsonStreamReader;
use crate::sqlcl::{SqlclConfig, SqlclProcess};
use crate::vortex_writer::VortexWriter;

pub struct Pipeline {
    config: SqlclConfig,
    batch_size: usize,
    auto_batch_rows: usize,
    skip_lobs: bool,
}

impl Pipeline {
    pub fn new(config: SqlclConfig, batch_size: usize, auto_batch_rows: usize, skip_lobs: bool) -> Self {
        Self { 
            config, 
            batch_size,
            auto_batch_rows,
            skip_lobs,
        }
    }

    /// Prepare SQL query for batching by wrapping with OFFSET/FETCH
    fn wrap_query_with_offset(&self, base_query: &str, offset: usize, fetch_rows: usize) -> String {
        // Remove comments (lines starting with --)
        let cleaned_query: Vec<&str> = base_query
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("--")
            })
            .collect();
        
        let base_query = cleaned_query.join("\n");
        
        // Remove trailing semicolon and whitespace
        let base_query = base_query.trim().trim_end_matches(';').trim();
        
        // Check if query already has OFFSET/FETCH
        let base_upper = base_query.to_uppercase();
        if base_upper.contains("OFFSET") && base_upper.contains("FETCH") {
            tracing::warn!("Query already contains OFFSET/FETCH, using as-is");
            return base_query.to_string();
        }
        
        // Wrap with OFFSET/FETCH (Oracle 12c+ syntax)
        format!(
            "SELECT * FROM (\n{}\n) \nOFFSET {} ROWS FETCH NEXT {} ROWS ONLY",
            base_query,
            offset,
            fetch_rows
        )
    }

    pub async fn run<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        sql_file: P,
        output_file: Q,
    ) -> Result<()> {
        // Read SQL query from file
        let sql_query = fs::read_to_string(&sql_file)
            .await
            .context("Failed to read SQL file")?;

        tracing::info!("SQL query loaded: {} bytes", sql_query.len());

        if self.auto_batch_rows == 0 {
            // Regular single-query mode
            self.run_single_query(&sql_query, output_file).await
        } else {
            // Auto-batching mode
            self.run_auto_batched(&sql_query, output_file).await
        }
    }

    async fn run_single_query<Q: AsRef<Path>>(
        &self,
        sql_query: &str,
        output_file: Q,
    ) -> Result<()> {
        tracing::info!("Starting pipeline (JSON format for type preservation)");
        tracing::info!("Batch size setting: {} rows (note: JSON loads all at once)", self.batch_size);

        // Spawn SQLcl process
        let mut sqlcl = SqlclProcess::spawn(&self.config, sql_query).await?;

        // Get stdout stream
        let stdout = sqlcl.stdout()
            .context("Failed to get SQLcl stdout")?;

        // Create JSON stream reader
        let mut json_reader = JsonStreamReader::new(stdout);

        // Read all JSON
        let records = json_reader.read_all_json().await?;

        tracing::info!("Loaded {} records from SQLcl", records.len());

        // Create Vortex writer
        let mut vortex_writer = VortexWriter::new(self.skip_lobs);

        // Process records
        let mut count = 0;
        for record in records {
            vortex_writer.add_record(record).await?;
            count += 1;

            if count % 1000 == 0 {
                tracing::info!("Processed {} records", count);
            }
        }

        tracing::info!("Total records processed: {}", count);

        // Flush to file
        if count > 0 {
            vortex_writer.flush(&output_file).await?;
        } else {
            tracing::warn!("No records to write");
        }

        // Wait for SQLcl to complete
        sqlcl.wait().await?;

        tracing::info!("Pipeline completed successfully");

        Ok(())
    }

    async fn run_auto_batched<Q: AsRef<Path>>(
        &self,
        base_sql_query: &str,
        output_file: Q,
    ) -> Result<()> {
        tracing::info!("Starting AUTO-BATCHING mode");
        tracing::info!("Batch size: {} rows per query", self.auto_batch_rows);

        // Create Vortex writer for all batches
        let mut vortex_writer = VortexWriter::new(self.skip_lobs);
        let mut total_count = 0;
        let mut batch_num = 0;
        let mut offset = 0;

        loop {
            batch_num += 1;
            
            // Create batched query
            let batched_query = self.wrap_query_with_offset(
                base_sql_query, 
                offset, 
                self.auto_batch_rows
            );

            tracing::info!("Batch {}: fetching rows {} to {}", 
                batch_num, offset, offset + self.auto_batch_rows - 1);
            
            // Execute SQLcl for this batch
            let mut sqlcl = SqlclProcess::spawn(&self.config, &batched_query).await?;
            
            let stdout = sqlcl.stdout()
                .context("Failed to get SQLcl stdout")?;

            let mut json_reader = JsonStreamReader::new(stdout);
            let records = json_reader.read_all_json().await?;
            
            let batch_size = records.len();
            tracing::info!("Batch {}: received {} records", batch_num, batch_size);

            // If no records, we've reached the end
            if batch_size == 0 {
                tracing::info!("No more records, stopping");
                break;
            }

            // Add records to vortex writer
            for record in records {
                vortex_writer.add_record(record).await?;
                total_count += 1;
            }

            // Wait for SQLcl to complete
            sqlcl.wait().await?;

            // If we got fewer records than requested, we're at the end
            if batch_size < self.auto_batch_rows {
                tracing::info!("Last batch (partial: {} records), stopping", batch_size);
                break;
            }

            offset += self.auto_batch_rows;
        }

        tracing::info!("Auto-batching complete: {} batches, {} total records", 
            batch_num, total_count);

        // Flush all records to file
        if total_count > 0 {
            vortex_writer.flush(&output_file).await?;
        } else {
            tracing::warn!("No records to write");
        }

        tracing::info!("Pipeline completed successfully");

        Ok(())
    }
}
