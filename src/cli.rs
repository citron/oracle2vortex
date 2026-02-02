use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "oracle2vortex")]
#[command(about = "Export Oracle tables to Vortex format via SQLcl streaming", long_about = None)]
pub struct CliArgs {
    /// Path to SQL query file
    #[arg(short = 'f', long)]
    pub sql_file: PathBuf,

    /// Output Vortex file path
    #[arg(short = 'o', long)]
    pub output: PathBuf,

    /// Oracle host
    #[arg(long)]
    pub host: String,

    /// Oracle port
    #[arg(long, default_value = "1521")]
    pub port: u16,

    /// Oracle user
    #[arg(short = 'u', long)]
    pub user: String,

    /// Oracle password
    #[arg(short = 'p', long)]
    pub password: String,

    /// Oracle SID or service name
    #[arg(long)]
    pub sid: String,

    /// Path to SQLcl executable
    #[arg(long, default_value = "sql")]
    pub sqlcl_path: PathBuf,

    /// Batch size for processing (rows per batch to keep memory usage constant)
    #[arg(long, default_value = "50000")]
    pub batch_size: usize,

    /// Auto-batch mode: split query into batches of N rows (0 = disabled, query runs as-is)
    /// When enabled, wraps query with OFFSET/FETCH and executes multiple times
    #[arg(long, default_value = "0")]
    pub auto_batch_rows: usize,

    /// Skip Oracle LOB types (CLOB, BLOB, NCLOB) - exclude them from the output
    #[arg(long, default_value = "false")]
    pub skip_lobs: bool,

    /// Use Oracle Thick driver (JDBC/OCI) instead of Thin driver
    /// Enables features like connection pooling, advanced security, and better performance
    #[arg(long, default_value = "false")]
    pub thick: bool,
}

impl CliArgs {
    pub fn validate(&self) -> anyhow::Result<()> {
        if !self.sql_file.exists() {
            anyhow::bail!("SQL file does not exist: {:?}", self.sql_file);
        }

        if !self.sql_file.is_file() {
            anyhow::bail!("SQL file path is not a file: {:?}", self.sql_file);
        }

        if self.output.exists() {
            tracing::warn!("Output file already exists and will be overwritten: {:?}", self.output);
        }

        Ok(())
    }
}
