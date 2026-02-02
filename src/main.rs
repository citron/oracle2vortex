mod cli;
mod csv_stream;  // Keep for future CSV mode option
mod json_stream;
mod pipeline;
mod sqlcl;
mod vortex_writer;

use anyhow::Result;
use clap::Parser;
use cli::CliArgs;
use pipeline::Pipeline;
use sqlcl::SqlclConfig;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    // Parse CLI arguments
    let args = CliArgs::parse();
    
    // Validate arguments
    args.validate()?;

    tracing::info!("Starting oracle2vortex");
    tracing::info!("SQL file: {:?}", args.sql_file);
    tracing::info!("Output file: {:?}", args.output);
    
    // Build connection string from args
    let connection_string = if let Some(ref connect_str) = args.connect_string {
        tracing::info!("Oracle: Using connection string");
        connect_str.clone()
    } else {
        let user = args.user.as_ref().unwrap();
        let password = args.password.as_ref().unwrap();
        let host = args.host.as_ref().unwrap();
        let sid = args.sid.as_ref().unwrap();
        tracing::info!("Oracle: {}@{}:{}/{}", user, host, args.port, sid);
        format!("{}/{}@//{}:{}/{}", user, password, host, args.port, sid)
    };
    
    if args.auto_batch_rows > 0 {
        tracing::info!("Mode: AUTO-BATCHING ({} rows per batch)", args.auto_batch_rows);
    } else {
        tracing::info!("Mode: Single query (JSON format preserves types)");
        if args.batch_size != 50000 {
            tracing::warn!("--batch-size parameter not used in single-query mode");
        }
    }
    
    if args.skip_lobs {
        tracing::info!("LOB filtering: ENABLED (CLOB, BLOB, NCLOB columns will be skipped)");
    }
    
    if args.thick {
        tracing::info!("Oracle driver: THICK (JDBC/OCI mode)");
    } else {
        tracing::info!("Oracle driver: THIN (default mode)");
    }

    // Create SQLcl configuration
    let config = SqlclConfig {
        connection_string,
        sqlcl_path: args.sqlcl_path.to_string_lossy().to_string(),
        thick: args.thick,
    };

    // Create and run pipeline
    let pipeline = Pipeline::new(config, args.batch_size, args.auto_batch_rows, args.skip_lobs);
    pipeline.run(&args.sql_file, &args.output).await?;

    tracing::info!("Successfully completed");

    Ok(())
}

