# oracle2vortex

> 🌍 **Available in 26 languages** - See [docs/TRANSLATIONS.md](docs/TRANSLATIONS.md) for all translations  
> 📖 **Read this in:** [FR](docs/locales/README.fr.md) | [DE](docs/locales/README.de.md) | [ES](docs/locales/README.es.md) | [IT](docs/locales/README.it.md) | [ZH](docs/locales/README.zh.md) | [+21 more](docs/TRANSLATIONS.md)

A CLI application that extracts Oracle tables to Vortex format via SQLcl with JSON streaming.

## Description

`oracle2vortex` allows exporting Oracle data using:
- **SQLcl** for connection and native JSON export
- **Streaming** to process data on-the-fly without waiting for export completion
- **Automatic conversion** to columnar Vortex format with schema inference

✅ **Project completed and tested in production** - Validated with a 417-column table on a real database.

## Prerequisites

- **Rust nightly** (required by Vortex crates)
- **SQLcl** installed (or specify path with `--sqlcl-path`)
- An accessible Oracle database

### Installing Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Installing SQLcl

Download SQLcl from: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Or on Linux:
```bash
# Example for installing in /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Installation

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

The executable will be available in `target/release/oracle2vortex`.

## Usage

### Basic syntax

```bash
oracle2vortex \
  --sql-file query.sql \
  --output data.vortex \
  --host localhost \
  --port 1521 \
  --user hr \
  --password mypassword \
  --sid ORCL
```

### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--sql-file` | `-f` | Path to SQL file containing the query | (required) |
| `--output` | `-o` | Output Vortex file path | (required) |
| `--host` | | Oracle host | (required) |
| `--port` | | Oracle port | 1521 |
| `--user` | `-u` | Oracle user | (required) |
| `--password` | `-p` | Oracle password | (required) |
| `--sid` | | Oracle SID or service name | (required) |
| `--sqlcl-path` | | Path to SQLcl executable | `sql` |
| `--auto-batch-rows` | | Number of rows per batch (0 = disabled) | 0 |
| `--skip-lobs` | | Skip Oracle LOB types (CLOB, BLOB, NCLOB) | false |

### Auto-Batching (Large Tables)

To process tables with millions or billions of rows with constant memory usage, use the `--auto-batch-rows` option:

```bash
# Process in batches of 50000 rows
oracle2vortex \
  -f query.sql \
  -o data.vortex \
  --host db.example.com \
  --port 1521 \
  -u hr \
  -p secret123 \
  --sid PROD \
  --auto-batch-rows 50000
```

**How it works:**
1. Automatically wraps your query with `OFFSET/FETCH`
2. Executes SQLcl multiple times (once per batch)
3. Accumulates all results in memory
4. Writes a single Vortex file containing all data

**Limitations:**
- Requires Oracle 12c+ (OFFSET/FETCH syntax)
- Your query must NOT already contain OFFSET/FETCH or ROWNUM
- Recommended: add ORDER BY for consistent ordering

**Memory:** With auto-batching, memory used = batch size × 2 (JSON + Vortex)  
Example: 50000 rows × 1 KB = 100 MB per batch (instead of loading the entire table)

**See also:** `BATCH_PROCESSING.md` and `README_LARGE_DATASETS.md` for more details.

### Skipping LOB Columns

Oracle LOB types (CLOB, BLOB, NCLOB) can be very large and may not be needed for analysis. Use `--skip-lobs` to exclude them:

```bash
# Skip LOB columns to reduce file size and improve performance
oracle2vortex \
  -f query.sql \
  -o data.vortex \
  --host db.example.com \
  --port 1521 \
  -u hr \
  -p secret123 \
  --sid PROD \
  --skip-lobs
```

**How it works:**
- Automatically detects and filters out columns containing LOB data
- LOBs are identified by size (> 4000 characters) or binary indicators
- The first record logged will show how many columns were skipped
- Reduces file size and memory usage significantly for tables with large text/binary fields

**Use cases:**
- Exporting metadata tables with description fields
- Working with tables containing XML or large JSON documents
- Focusing on structured data while ignoring binary content
- Performance optimization for tables with many large columns

### Example with SQL file

Create a `query.sql` file:

```sql
SELECT 
    employee_id,
    first_name,
    last_name,
    salary,
    hire_date
FROM employees
WHERE department_id = 50;
```

Then execute:

```bash
oracle2vortex \
  -f query.sql \
  -o employees.vortex \
  --host db.example.com \
  --port 1521 \
  -u hr \
  -p secret123 \
  --sid PROD
```

## Architecture

```
┌─────────────┐
│  SQL File   │
│             │
└──────┬──────┘
       │
       v
┌──────────────────────────┐
│  oracle2vortex CLI       │
│  (Clap argument parser)  │
└──────────┬───────────────┘
           │
           v
┌──────────────────────────┐
│  SQLcl Process           │
│  (CONNECT, SET FORMAT)   │
└──────────┬───────────────┘
           │ JSON: {"results":[{"items":[...]}]}
           v
┌──────────────────────────┐
│  JSON Stream Parser      │
│  (extraction + parsing)  │
└──────────┬───────────────┘
           │ Vec<serde_json::Value>
           v
┌──────────────────────────┐
│  Vortex Writer           │
│  (schema inference +     │
│   ArrayData construction)│
└──────────┬───────────────┘
           │ Vortex format
           v
┌──────────────────────────┐
│  .vortex File            │
│  (columnar binary)       │
└──────────────────────────┘
```

## How it works

1. **SQL Reading**: The SQL file is loaded into memory
2. **SQLcl Launch**: Process starts with Oracle connection
3. **Session configuration**:
   - `SET SQLFORMAT JSON` for JSON export
   - `SET NLS_NUMERIC_CHARACTERS='.,';` for decimal point compatibility
   - `SET NLS_DATE_FORMAT='YYYY-MM-DD"T"HH24:MI:SS';` for ISO 8601 date format
   - `SET NLS_TIMESTAMP_FORMAT='YYYY-MM-DD"T"HH24:MI:SS.FF';` for ISO 8601 timestamp format
   - Additional settings for optimized export (FEEDBACK OFF, TIMING OFF, TERMOUT OFF, etc.)
4. **Query execution**: The SQL query is sent via stdin
5. **Output capture**: Complete reading of JSON stdout
6. **JSON extraction**: Isolation of the `{"results":[{"items":[...]}]}` structure
7. **Schema inference**: The Vortex schema is automatically deduced from the first record
8. **Record conversion**: Each JSON object is transformed into Vortex columns
9. **File writing**: Binary Vortex file created with Tokio session

## Supported data types

Automatic Oracle to Vortex type mapping with optimal storage:

### Complete Type Mapping

| Oracle Type | JSON Export | Vortex Type | Storage | Notes |
|-------------|-------------|-------------|---------|-------|
| **Temporal Types** |
| `DATE` | `"2024-01-15"` | `Extension(Date)` | I32 | Days since 1970-01-01 |
| `TIMESTAMP` | `"2024-01-15T14:30:45.123456"` | `Extension(Timestamp)` | I64 | Microseconds since epoch |
| `TIMESTAMP WITH TIME ZONE` | `"2024-01-15T14:30:45.123456 +02:00"` | `Extension(Timestamp)` | I64 | Converted to UTC, timezone in metadata |
| `TIMESTAMP WITH LOCAL TZ` | Same as TIMESTAMP WITH TZ | `Extension(Timestamp)` | I64 | Converted to session timezone then UTC |
| `INTERVAL DAY TO SECOND` | `"+02 02:30:00.123456"` | `Primitive(I64)` | I64 | Total microseconds |
| `INTERVAL YEAR TO MONTH` | `"+01-06"` | `Primitive(I32)` | I32 | Total months |
| **Numeric Types** |
| `NUMBER` (integer) | `123` | `Primitive(I64)` | I64 | Whole numbers |
| `NUMBER` (decimal) | `123.45` | `Primitive(F64)` | F64 | Floating point |
| `BINARY_FLOAT` | `3.14` | `Primitive(F64)` | F64 | IEEE 754 single precision |
| `BINARY_DOUBLE` | `2.718` | `Primitive(F64)` | F64 | IEEE 754 double precision |
| **Character Types** |
| `VARCHAR2`, `NVARCHAR2` | `"text"` | `Utf8` | VarBinArray | Variable-length strings |
| `CHAR`, `NCHAR` | `"text"` | `Utf8` | VarBinArray | Fixed-length (padded) |
| `CLOB`, `NCLOB` | `"long text"` | `Utf8` or skip | VarBinArray | Use `--skip-lobs` to exclude |
| **Binary Types** |
| `RAW`, `LONG RAW` | `"DEADBEEF"` (hex) | `Binary` | VarBinArray | Detected if ≥8 hex chars |
| `BLOB` | `"hex string"` | `Binary` or skip | VarBinArray | Use `--skip-lobs` to exclude |
| **Structured Types** |
| `JSON` (Oracle 21c+) | `"{\"key\":\"value\"}"` | `Utf8` | VarBinArray | Validated JSON, kept as string |
| `XMLTYPE` | `"<root/>"` | `Utf8` | VarBinArray | XML as string |
| **Other Types** |
| `ROWID`, `UROWID` | `"AAABbbCCC..."` | `Utf8` | VarBinArray | Oracle-specific format |
| `BOOLEAN` (via JSON) | `true`/`false` | `Bool` | BitBuffer | Native boolean |
| `null` | `null` | (inferred) | - | Nullable variant of detected type |

**Note**: All types are nullable to handle Oracle NULL values.

### Temporal Types with Timezone Support

Oracle temporal columns are automatically detected and converted to native Vortex temporal types:

- **DATE** (YYYY-MM-DD): Stored as `Extension(vortex.date)` with I32 backing (days since 1970-01-01)
- **TIMESTAMP** (YYYY-MM-DDTHH:MI:SS[.ffffff]): Stored as `Extension(vortex.timestamp)` with I64 backing (microseconds since epoch)
- **TIMESTAMP WITH TIME ZONE**: Stored as `Extension(vortex.timestamp)` with timezone metadata, **converted to UTC** for storage

SQLcl is configured to output these formats using:
```sql
ALTER SESSION SET NLS_DATE_FORMAT = 'YYYY-MM-DD"T"HH24:MI:SS';
ALTER SESSION SET NLS_TIMESTAMP_FORMAT = 'YYYY-MM-DD"T"HH24:MI:SS.FF';
ALTER SESSION SET NLS_TIMESTAMP_TZ_FORMAT = 'YYYY-MM-DD"T"HH24:MI:SS.FF TZH:TZM';
```

### Binary Data (RAW/BLOB)

Oracle RAW and BLOB types are detected when exported as hexadecimal strings (minimum 8 characters, uppercase):
- Automatically converted from hex to binary
- Stored efficiently in `DType::Binary` using `VarBinArray`
- Example: `HEXTORAW('DEADBEEF')` → binary `[0xDE, 0xAD, 0xBE, 0xEF]`

This ensures dates, timestamps, and binary data are preserved as typed data, not strings, enabling efficient queries and operations.

## Logging and debugging

The application uses `tracing` for logs. Messages are displayed on stderr with log level.

Logs include:
- Oracle connection
- Number of processed records
- Inferred schema
- Errors and warnings

## Verifying generated Vortex files

To verify generated files, use the `vx` tool:

```bash
# Install vx (Vortex CLI tool)
cargo install vortex-vx

# Browse a Vortex file
vx browse output.vortex

# Display metadata
vx info output.vortex
```

## Limitations and considerations

- **Complex types**: Nested JSON objects and arrays are serialized to strings
- **In-memory buffer**: Records are currently buffered before writing (future optimization possible)
- **Fixed schema**: Inferred from first record only (subsequent records must match)
- **Security**: Password is passed as CLI argument (visible with `ps`). Use environment variables in production.
- **LOB types**: By default, LOB columns (CLOB, BLOB, NCLOB) are included. Use `--skip-lobs` to exclude them for better performance and smaller file sizes.

## Development

### Debug build

```bash
cargo build
```

### Release build

```bash
cargo build --release
```

The binary will be in `target/release/oracle2vortex` (~46 MB in release).

### Tests

```bash
cargo test
```

### Manual tests

Test files with credentials are in `tests_local/` (gitignored):

```bash
# Create test queries
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Execute
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## License

Copyright (c) 2026 William Gacquer

This project is licensed under EUPL-1.2 (European Union Public Licence v. 1.2).

**IMPORTANT - Commercial use restriction:**  
Commercial use of this software is prohibited without prior written agreement with the author.  
For any commercial license request, please contact: **oracle2vortex@amilto.com**

See the [LICENSE](LICENSE) file for the complete license text.

## Author

**William Gacquer**  
Contact: oracle2vortex@amilto.com

## Test history

The project has been validated on a production Oracle database:

- ✅ **Simple test**: 10 records, 3 columns → 5.5 KB
- ✅ **Complex test**: 100 records, 417 columns → 1.3 MB
- ✅ **Validation**: Files readable with `vx browse` (Vortex v0.58)

## Project structure

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # This file
├── IMPLEMENTATION.md       # Technical documentation
├── .gitignore             # Excludes tests_local/ and credentials
├── src/
│   ├── main.rs            # Entry point with tokio runtime
│   ├── cli.rs             # Clap argument parsing
│   ├── sqlcl.rs           # SQLcl process with CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # JSON→Vortex conversion (API 0.58)
│   └── pipeline.rs        # Complete orchestration
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Sample query
└── tests_local/           # Tests with credentials (gitignored)
```

## Main dependencies

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Resources

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
