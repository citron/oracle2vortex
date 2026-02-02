# Temporal and Binary Types Support - Implementation Notes

## Overview

`oracle2vortex` automatically detects and preserves Oracle temporal types (DATE, TIMESTAMP, TIMESTAMP WITH TIME ZONE) and binary types (RAW, BLOB) as native Vortex types instead of strings. This provides optimal storage efficiency, type safety, and query performance.

## Version History

- **v0.1.0**: Initial DATE and TIMESTAMP support
- **v0.2.0**: Added TIMESTAMP WITH TIME ZONE and RAW/Binary support

## Changes Made

### 1. Dependencies Added

- `jiff = "0.1"` - For ISO 8601 date/timestamp parsing (including timezone support)
- `vortex-dtype` with feature `"arrow"` - Enables temporal extension types

### 2. SQLcl Configuration Enhanced

The following commands are now automatically sent to SQLcl on connection:

```sql
-- Temporal format configuration (ISO 8601)
ALTER SESSION SET NLS_DATE_FORMAT = 'YYYY-MM-DD"T"HH24:MI:SS';
ALTER SESSION SET NLS_TIMESTAMP_FORMAT = 'YYYY-MM-DD"T"HH24:MI:SS.FF';
ALTER SESSION SET NLS_TIMESTAMP_TZ_FORMAT = 'YYYY-MM-DD"T"HH24:MI:SS.FF TZH:TZM';

-- Export optimization
SET FEEDBACK OFF       -- Suppress "X rows selected"
SET TIMING OFF         -- Suppress execution time
SET VERIFY OFF         -- Suppress variable substitution
SET TERMOUT OFF        -- Suppress screen output (faster)
SET TRIMSPOOL ON       -- Remove trailing whitespace
SET ENCODING UTF-8     -- Force UTF-8 encoding
```

### 3. Type Detection Logic

In `vortex_writer.rs`, the `infer_dtype()` function now:

1. **Detects ISO 8601 timestamps with timezone** (YYYY-MM-DDTHH:MI:SS[.fff] +/-HH:MM or Z)
   - Maps to `Extension(vortex.timestamp)` with I64 backing and timezone metadata
   - Stores as microseconds since Unix epoch (UTC)
   - Timezone preserved in metadata

2. **Detects ISO 8601 dates** (YYYY-MM-DD format)
   - Maps to `Extension(vortex.date)` with I32 backing
   - Stores as days since 1970-01-01

3. **Detects ISO 8601 timestamps without timezone** (YYYY-MM-DDTHH:MI:SS[.ffffff])
   - Maps to `Extension(vortex.timestamp)` with I64 backing
   - Stores as microseconds since Unix epoch
   - Supports fractional seconds up to 6 digits (microseconds)

4. **Detects hexadecimal binary data** (RAW/BLOB as hex strings)
   - Maps to `DType::Binary`
   - Converts from hex string to raw bytes
   - Minimum 8 characters to avoid false positives

5. **Falls back to Utf8** for other string values

### 4. Parsing Functions

Several parsing functions were added:

```rust
fn parse_date_to_days(s: &str) -> Option<i32>
```
- Parses YYYY-MM-DD format
- Returns days since 1970-01-01
- Example: "2024-01-01" → 19723

```rust
fn parse_timestamp_to_micros(s: &str) -> Option<i64>
```
- Parses YYYY-MM-DDTHH:MI:SS[.ffffff] format
- Returns microseconds since Unix epoch
- Handles fractional seconds correctly
- Example: "1970-01-01T00:00:01.500000" → 1,500,000

```rust
fn parse_oracle_tz_format(s: &str) -> Option<i64>
```
- Parses YYYY-MM-DDTHH:MI:SS[.ffffff] +/-HH:MM or Z
- Converts to UTC by subtracting timezone offset
- Returns microseconds since Unix epoch (UTC)
- Example: "2024-01-15T14:00:00 +02:00" → UTC 12:00:00

```rust
fn parse_tz_offset(tz: &str) -> Option<i64>
```
- Parses timezone offset string (e.g., "+02:00", "-05:30")
- Returns offset in seconds
- Example: "+02:00" → 7200

```rust
fn hex_to_binary(s: &str) -> Option<Vec<u8>>
```
- Converts hexadecimal string to binary bytes
- Example: "DEADBEEF" → [0xDE, 0xAD, 0xBE, 0xEF]

### 5. Column Array Construction

Added match branches in the `flush()` method to handle:

- `DType::Extension(DATE_ID)` - Constructs PrimitiveArray<i32> from parsed dates
- `DType::Extension(TIMESTAMP_ID)` - Constructs PrimitiveArray<i64> from parsed timestamps (with or without timezone)
- `DType::Binary` - Constructs VarBinArray from hex-decoded binary data

## Testing

### Unit Tests

Added 17 comprehensive unit tests in `src/vortex_writer.rs`:

**Temporal Tests:**
1. `test_is_iso_date()` - Validates date pattern detection
2. `test_is_iso_timestamp()` - Validates timestamp pattern detection  
3. `test_parse_date_to_days()` - Validates date conversion
4. `test_parse_timestamp_to_micros()` - Validates timestamp conversion
5. `test_infer_dtype_date()` - Validates type inference for dates
6. `test_infer_dtype_timestamp()` - Validates type inference for timestamps

**Timezone Tests:**
7. `test_is_iso_timestamp_tz()` - Validates TZ timestamp detection
8. `test_extract_timezone()` - Validates timezone extraction
9. `test_parse_tz_offset()` - Validates offset parsing
10. `test_parse_oracle_tz_format()` - Validates TZ conversion to UTC

**Binary Tests:**
11. `test_is_hex_string()` - Validates hex string detection
12. `test_hex_to_binary()` - Validates hex to binary conversion
13. `test_infer_dtype_binary()` - Validates Binary type inference
14. `test_infer_dtype_timestamp_tz()` - Validates TZ timestamp inference

**General Tests:**
15. `test_infer_dtype_string()` - Ensures non-temporal strings still work
16. `test_infer_dtype_number()` - Ensures numeric types still work
17. `test_infer_dtype_float()` - Ensures float types still work

All tests pass: ✅ `cargo test` - **17 passed; 0 failed**

### Integration Test Queries

**`tests_local/test_temporal.sql`** - Temporal types:
- Pure DATE literals
- TIMESTAMP literals with fractional seconds
- TIMESTAMP WITH TIME ZONE (positive and negative offsets)
- SYSDATE and SYSTIMESTAMP
- RAW/Binary data (HEXTORAW)
- NULL temporal values
- Mixed with other data types

**`tests_local/test_all_types.sql`** - Comprehensive coverage:
- All major Oracle types
- Edge cases and boundary values
- Multiple rows for consistency testing

## Benefits

1. **Type Safety**: Dates/timestamps are now typed, not strings
2. **Storage Efficiency**: More compact representation (4 or 8 bytes vs variable-length string)
3. **Query Performance**: Temporal operations can be optimized
4. **Standards Compliance**: ISO 8601 format ensures compatibility
5. **Precision**: Microsecond precision for timestamps (Oracle supports up to nanoseconds, but we chose microseconds for compatibility)

## Limitations

1. **Timezone Support**: Currently timestamps without timezone only
   - Oracle TIMESTAMP WITH TIME ZONE columns will be converted but timezone info is not preserved
   - Future enhancement could add timezone support using `TemporalMetadata::Timestamp(unit, Some(tz))`

2. **Oracle TIME Type**: Not common in Oracle (typically use DATE), not explicitly handled

3. **Non-ISO Formats**: Only ISO 8601 formats are detected as temporal types
   - Other date string formats fall back to Utf8
   - This is intentional to avoid false positives

## Future Enhancements

1. Add timezone support for TIMESTAMP WITH TIME ZONE
2. Add configuration option to disable temporal type detection (force all to strings)
3. Support Oracle INTERVAL types
4. Add more temporal format patterns beyond ISO 8601

## Breaking Changes

**None.** This is a backward-compatible enhancement:
- Existing string-based date handling still works
- Only dates in ISO 8601 format are upgraded to temporal types
- Fallback to string ensures no data is lost
