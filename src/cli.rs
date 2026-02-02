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

    /// Complete Oracle connection string (user/password@connect_identifier)
    /// Example: hr/mypass@//localhost:1521/ORCL
    /// When provided, --user, --password, --host, --port, and --sid are ignored
    #[arg(short = 'c', long)]
    pub connect_string: Option<String>,

    /// Oracle host (required if --connect-string not provided)
    #[arg(long, required_unless_present = "connect_string")]
    pub host: Option<String>,

    /// Oracle port
    #[arg(long, default_value = "1521")]
    pub port: u16,

    /// Oracle user (required if --connect-string not provided)
    #[arg(short = 'u', long, required_unless_present = "connect_string")]
    pub user: Option<String>,

    /// Oracle password (required if --connect-string not provided)
    #[arg(short = 'p', long, required_unless_present = "connect_string")]
    pub password: Option<String>,

    /// Oracle SID or service name (required if --connect-string not provided)
    #[arg(long, required_unless_present = "connect_string")]
    pub sid: Option<String>,

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

        // Validate that we have either connect_string OR all individual components
        if self.connect_string.is_none() && 
           (self.user.is_none() || self.password.is_none() || self.host.is_none() || self.sid.is_none()) {
            anyhow::bail!("Either --connect-string or all of (--user, --password, --host, --sid) must be provided");
        }

        Ok(())
    }
}
