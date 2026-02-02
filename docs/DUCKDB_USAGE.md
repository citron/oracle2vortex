# Using Vortex Files with DuckDB

## Overview

`oracle2vortex` exports Oracle data optimally using native binary types for dates, timestamps, and other specialized Oracle types. When viewing these files in DuckDB, some types require conversion from their storage format to human-readable values.

## Temporal Types (DATE, TIMESTAMP)

### Storage Format

- **DATE**: Stored as I32 (days since Unix epoch: 1970-01-01)
- **TIMESTAMP**: Stored as I64 (microseconds since Unix epoch)
- **TIMESTAMP WITH TIME ZONE**: Stored as I64 (microseconds since Unix epoch in UTC)

### Why Numeric Storage?

This provides:
- **60-78% storage reduction** compared to strings
- **Faster filtering** (numeric comparisons vs string parsing)
- **Type safety** (impossible to store invalid dates)
- **Sortable** without parsing

### Viewing in DuckDB

DuckDB doesn't automatically recognize Vortex extension types, so dates appear as numbers. Convert them in your queries:

#### DATE Conversion

```sql
SELECT 
    patient_id,
    -- Convert I32 days to DATE
    (DATE '1970-01-01' + INTERVAL (birth_date) DAYS) AS birth_date,
    patient_name
FROM 'patients.vortex';
```

**Example:**
```sql
-- birth_date stored as: 19754 (days since epoch)
-- Converts to: 2024-02-02
```

#### TIMESTAMP Conversion

```sql
SELECT 
    event_id,
    -- Convert I64 microseconds to TIMESTAMP
    to_timestamp(event_time / 1000000.0) AS event_time,
    event_type
FROM 'events.vortex';
```

**Example:**
```sql
-- event_time stored as: 1706889600000000 (microseconds)
-- Converts to: 2024-02-02 12:00:00
```

#### TIMESTAMP WITH TIME ZONE

```sql
SELECT 
    transaction_id,
    -- Already in UTC, just convert microseconds to timestamp
    to_timestamp(transaction_ts / 1000000.0) AS transaction_ts_utc,
    amount
FROM 'transactions.vortex';
```

**Note:** Timezone information is preserved in Vortex metadata. The value is stored as UTC.

## Creating Readable Views

For convenience, create views with pre-converted dates:

```sql
-- Create view with all conversions
CREATE VIEW patients_readable AS
SELECT 
    patient_id,
    (DATE '1970-01-01' + INTERVAL (birth_date) DAYS) AS birth_date,
    to_timestamp(admission_ts / 1000000.0) AS admission_timestamp,
    to_timestamp(discharge_ts / 1000000.0) AS discharge_timestamp,
    patient_name,
    diagnosis
FROM read_vortex('patients.vortex');

-- Now query with readable dates
SELECT * 
FROM patients_readable 
WHERE birth_date BETWEEN '1990-01-01' AND '2000-12-31'
  AND admission_timestamp > '2024-01-01';
```

## INTERVAL Types

### Storage Format

- **INTERVAL DAY TO SECOND**: Stored as I64 (total microseconds)
- **INTERVAL YEAR TO MONTH**: Stored as I32 (total months)

### Conversion Examples

#### INTERVAL DAY TO SECOND

```sql
SELECT 
    task_id,
    -- Convert microseconds to human-readable interval
    INTERVAL (duration / 1000000.0) SECONDS AS duration_readable
FROM 'tasks.vortex';
```

Or extract components:

```sql
SELECT 
    task_id,
    duration / 1000000 / 86400 AS days,
    (duration / 1000000 % 86400) / 3600 AS hours,
    (duration / 1000000 % 3600) / 60 AS minutes,
    duration / 1000000 % 60 AS seconds
FROM 'tasks.vortex';
```

#### INTERVAL YEAR TO MONTH

```sql
SELECT 
    subscription_id,
    subscription_duration / 12 AS years,
    subscription_duration % 12 AS months
FROM 'subscriptions.vortex';
```

## Binary Types (RAW, BLOB)

Binary data is stored efficiently (50% reduction vs hex strings). DuckDB displays it as binary:

```sql
SELECT 
    file_id,
    -- View as hex string if needed
    hex(file_content) AS content_hex,
    -- Get length in bytes
    octet_length(file_content) AS size_bytes
FROM 'files.vortex';
```

## Complete Example

```sql
-- Full patient data with all conversions
CREATE VIEW patient_data AS
SELECT 
    patient_id,
    patient_name,
    (DATE '1970-01-01' + INTERVAL (birth_date) DAYS) AS birth_date,
    to_timestamp(admission_ts / 1000000.0) AS admission_timestamp,
    to_timestamp(discharge_ts / 1000000.0) AS discharge_timestamp,
    INTERVAL ((discharge_ts - admission_ts) / 1000000.0) SECONDS AS stay_duration,
    diagnosis,
    hex(medical_image) AS image_hex
FROM read_vortex('patient_records.vortex');

-- Query with natural syntax
SELECT 
    patient_name,
    birth_date,
    admission_timestamp,
    EXTRACT(DAY FROM stay_duration) AS days_hospitalized
FROM patient_data
WHERE birth_date > '1980-01-01'
  AND stay_duration > INTERVAL '7' DAYS
ORDER BY admission_timestamp DESC;
```

## Performance Considerations

### Filtering Performance

Even though conversion is needed for display, **filtering on raw values is still faster**:

```sql
-- FAST: Filter on raw I32 value (no conversion)
SELECT * FROM 'patients.vortex'
WHERE birth_date > 10957;  -- 2000-01-01 as days

-- SLOWER: Filter on converted value
SELECT * FROM 'patients.vortex'
WHERE (DATE '1970-01-01' + INTERVAL (birth_date) DAYS) > '2000-01-01';
```

**Recommendation**: For complex queries with many filters, use raw values in WHERE clauses and convert only in SELECT.

```sql
-- Optimal: Filter raw, convert for display
SELECT 
    patient_id,
    (DATE '1970-01-01' + INTERVAL (birth_date) DAYS) AS birth_date
FROM 'patients.vortex'
WHERE birth_date > 10957  -- Raw value filter
  AND birth_date < 19723  -- 2024-01-01
LIMIT 100;
```

### Indexing

DuckDB can index the raw numeric values efficiently:

```sql
-- Create table from Vortex for indexing
CREATE TABLE patients_indexed AS 
SELECT * FROM 'patients.vortex';

-- Index on raw date value (fast!)
CREATE INDEX idx_birth_date ON patients_indexed(birth_date);

-- Queries use index on raw values
SELECT * FROM patients_indexed WHERE birth_date > 10957;
```

## Type Reference Quick Guide

| Oracle Type               | Vortex Storage     | DuckDB Conversion                                     |
|---------------------------|--------------------|----------------------------------------------------|
| DATE                      | I32 (days)         | `DATE '1970-01-01' + INTERVAL (col) DAYS`         |
| TIMESTAMP                 | I64 (microseconds) | `to_timestamp(col / 1000000.0)`                   |
| TIMESTAMP WITH TIME ZONE  | I64 (UTC micros)   | `to_timestamp(col / 1000000.0)`                   |
| INTERVAL DAY TO SECOND    | I64 (microseconds) | `INTERVAL (col / 1000000.0) SECONDS`              |
| INTERVAL YEAR TO MONTH    | I32 (months)       | `INTERVAL (col) MONTHS`                           |
| RAW / BLOB                | Binary             | `hex(col)` (to view as hex)                       |
| JSON                      | UTF-8 String       | Use as-is or `json(col)`                          |

## Epoch Reference Values

For quick reference when working with raw values:

```sql
-- Common date milestones as I32 days
-- 1970-01-01 = 0
-- 1980-01-01 = 3653
-- 1990-01-01 = 7305
-- 2000-01-01 = 10957
-- 2010-01-01 = 14610
-- 2020-01-01 = 18262
-- 2024-01-01 = 19723
-- 2025-01-01 = 20088
-- 2026-01-01 = 20453
```

## Future Improvements

DuckDB may add native support for Vortex extension types in future versions, eliminating the need for manual conversion. Check DuckDB release notes for updates.

## See Also

- [Oracle Type Mapping](ORACLE_TYPE_MAPPING.md) - Complete type conversion reference
- [Temporal Types Implementation](TEMPORAL_TYPES.md) - Technical details
- [DuckDB Documentation](https://duckdb.org/docs/) - Official DuckDB docs
