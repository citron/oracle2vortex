# Using Vortex Files with DuckDB

This directory contains helper files to make working with Vortex files in DuckDB easier.

## Quick Start

### 1. Start DuckDB

```bash
duckdb
```

### 2. Load Helper Macros

```sql
.read examples/duckdb_helpers.sql
```

You should see:
```
┌─────────────────────────────────────────────┐
│ DuckDB Vortex helper macros loaded!         │
└─────────────────────────────────────────────┘
```

### 3. Query Your Vortex Files

```sql
-- Convert dates and timestamps
SELECT 
    employee_id,
    first_name,
    vortex_to_date(hire_date) AS hire_date,
    vortex_to_timestamp(last_update) AS last_update
FROM 'employees.vortex';
```

## Files in This Directory

| File | Description |
|------|-------------|
| `duckdb_helpers.sql` | **Load this first!** Contains reusable macros for converting Vortex temporal types |
| `duckdb_usage_examples.sql` | Complete examples: patients, events, subscriptions, tasks, binary data, etc. |
| `README_DUCKDB.md` | This file |
| `sample_query.sql` | Oracle query example for oracle2vortex |

## Available Macros

After loading `duckdb_helpers.sql`, you have these macros:

### Temporal Conversions

```sql
-- Convert DATE (I32 days) to readable date
vortex_to_date(days)

-- Convert TIMESTAMP (I64 microseconds) to readable timestamp
vortex_to_timestamp(micros)

-- Convert TIMESTAMP WITH TIME ZONE (I64 UTC micros) to timestamptz
vortex_to_timestamptz(micros)
```

### Interval Conversions

```sql
-- Convert INTERVAL DAY TO SECOND (I64 microseconds) to interval
vortex_to_interval_ds(micros)

-- Convert INTERVAL YEAR TO MONTH (I32 months) to interval
vortex_to_interval_ym(months)

-- Extract components from INTERVAL DAY TO SECOND
interval_ds_days(micros)      -- Get days
interval_ds_hours(micros)     -- Get hours
interval_ds_minutes(micros)   -- Get minutes
interval_ds_seconds(micros)   -- Get seconds

-- Extract components from INTERVAL YEAR TO MONTH
interval_ym_years(months)     -- Get years
interval_ym_months(months)    -- Get remaining months
```

### Binary Data

```sql
-- Convert binary to hex string
vortex_to_hex(binary_data)

-- Get binary size in bytes
vortex_binary_size(binary_data)
```

### Performance Helpers

```sql
-- Convert DuckDB DATE to Vortex format (for fast WHERE clauses)
date_to_vortex(dt)

-- Convert DuckDB TIMESTAMP to Vortex format (for fast WHERE clauses)
timestamp_to_vortex(ts)
```

## Common Patterns

### Pattern 1: Create Readable View

```sql
-- Load macros
.read examples/duckdb_helpers.sql

-- Create view with converted columns
CREATE VIEW patients AS
SELECT 
    patient_id,
    patient_name,
    vortex_to_date(birth_date) AS birth_date,
    vortex_to_timestamp(admission_ts) AS admission_timestamp,
    diagnosis
FROM 'patients.vortex';

-- Query naturally
SELECT * FROM patients WHERE birth_date > '2000-01-01';
```

### Pattern 2: Performance-Optimized Filtering

```sql
-- FAST: Filter raw values, convert for display
SELECT 
    patient_id,
    vortex_to_date(birth_date) AS birth_date
FROM 'patients.vortex'
WHERE birth_date > date_to_vortex('2000-01-01')  -- Filter on I32
LIMIT 100;

-- SLOW: Convert then filter (don't do this!)
SELECT 
    patient_id,
    vortex_to_date(birth_date) AS birth_date
FROM 'patients.vortex'
WHERE vortex_to_date(birth_date) > '2000-01-01'  -- Converts EVERY row!
LIMIT 100;
```

### Pattern 3: Complex Date Calculations

```sql
-- Calculate age and length of stay
SELECT 
    patient_id,
    vortex_to_date(birth_date) AS birth_date,
    CAST(date_diff('year', 
        vortex_to_date(birth_date), 
        current_date
    ) AS INTEGER) AS age,
    vortex_to_timestamp(admission_ts) AS admission,
    vortex_to_timestamp(discharge_ts) AS discharge,
    interval_ds_days(discharge_ts - admission_ts) AS days_hospitalized
FROM 'patients.vortex';
```

### Pattern 4: Export to Standard Formats

```sql
-- Export to CSV with readable dates
COPY (
    SELECT 
        employee_id,
        first_name,
        vortex_to_date(hire_date) AS hire_date,
        salary
    FROM 'employees.vortex'
) TO 'employees_readable.csv' (HEADER);

-- Export to Parquet
COPY (
    SELECT 
        patient_id,
        vortex_to_date(birth_date) AS birth_date,
        vortex_to_timestamp(admission_ts) AS admission
    FROM 'patients.vortex'
) TO 'patients_readable.parquet';
```

## Complete Examples

See `duckdb_usage_examples.sql` for complete examples of:

1. **Employee data** - Simple dates and filtering
2. **Patient records** - Multiple temporal columns, duration calculations
3. **Event logs** - Timestamps with timezone, hourly aggregations
4. **Subscriptions** - INTERVAL YEAR TO MONTH handling
5. **Tasks** - INTERVAL DAY TO SECOND with component extraction
6. **Binary data** - RAW/BLOB viewing and size analysis
7. **Multi-table joins** - Combining multiple Vortex files
8. **Export workflows** - Converting to CSV/Parquet/JSON
9. **Performance comparisons** - Fast vs slow query patterns

## Epoch Reference Values

For quick manual filtering, here are common dates as I32 values:

```sql
-- 2000-01-01 = 10957
-- 2010-01-01 = 14610
-- 2020-01-01 = 18262
-- 2024-01-01 = 19723
-- 2025-01-01 = 20088
-- 2026-01-01 = 20453

-- Example: Fast filter using literal value
SELECT * FROM 'patients.vortex' WHERE birth_date > 10957;  -- After 2000-01-01
```

## List Available Macros

```sql
SELECT show_vortex_macros();
```

## Troubleshooting

### Dates appear as numbers

You forgot to load the helper macros:
```sql
.read examples/duckdb_helpers.sql
```

### "Macro not found" error

Make sure you're in the correct directory or use absolute path:
```sql
.read /full/path/to/oracle2vortex/examples/duckdb_helpers.sql
```

### Slow queries

Make sure you're filtering on raw values before converting:
```sql
-- Good
WHERE birth_date > date_to_vortex('2000-01-01')

-- Bad
WHERE vortex_to_date(birth_date) > '2000-01-01'
```

## See Also

- [Complete DuckDB Usage Guide](../docs/DUCKDB_USAGE.md) - Detailed documentation
- [Oracle Type Mapping](../docs/ORACLE_TYPE_MAPPING.md) - How Oracle types map to Vortex
- [Main README](../README.md) - oracle2vortex documentation

## Support

For issues or questions:
- Email: oracle2vortex@amilto.com
- Check documentation: `docs/DUCKDB_USAGE.md`
