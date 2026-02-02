# Temporal Types Support - Implementation Notes

## Overview

As of this version, `oracle2vortex` automatically detects and preserves Oracle DATE and TIMESTAMP columns as native Vortex temporal types instead of strings.

## Changes Made

### 1. Dependencies Added

- `jiff = "0.1"` - For ISO 8601 date/timestamp parsing
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

1. **Detects ISO 8601 dates** (YYYY-MM-DD format)
   - Maps to `Extension(vortex.date)` with I32 backing
   - Stores as days since 1970-01-01

2. **Detects ISO 8601 timestamps** (YYYY-MM-DDTHH:MI:SS[.ffffff] format)
   - Maps to `Extension(vortex.timestamp)` with I64 backing
   - Stores as microseconds since Unix epoch
   - Supports fractional seconds up to 6 digits (microseconds)

3. **Falls back to Utf8** for other string values

### 4. Parsing Functions

Two new parsing functions were added:

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

### 5. Column Array Construction

Added two new match branches in the `flush()` method to handle:

- `DType::Extension(DATE_ID)` - Constructs PrimitiveArray<i32> from parsed dates
- `DType::Extension(TIMESTAMP_ID)` - Constructs PrimitiveArray<i64> from parsed timestamps

## Testing

### Unit Tests

Added 9 comprehensive unit tests in `src/vortex_writer.rs`:

1. `test_is_iso_date()` - Validates date pattern detection
2. `test_is_iso_timestamp()` - Validates timestamp pattern detection
3. `test_parse_date_to_days()` - Validates date conversion
4. `test_parse_timestamp_to_micros()` - Validates timestamp conversion
5. `test_infer_dtype_date()` - Validates type inference for dates
6. `test_infer_dtype_timestamp()` - Validates type inference for timestamps
7. `test_infer_dtype_string()` - Ensures non-temporal strings still work
8. `test_infer_dtype_number()` - Ensures numeric types still work
9. `test_infer_dtype_float()` - Ensures float types still work

All tests pass: ✅ `cargo test` - 9 passed; 0 failed

### Integration Test Query

Created `tests_local/test_temporal.sql` with examples of:
- Pure DATE literals
- TIMESTAMP literals with fractional seconds
- SYSDATE and SYSTIMESTAMP
- NULL temporal values
- Mixed with other data types

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
