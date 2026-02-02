-- ========================================================================
-- DuckDB Helper Macros for Vortex Temporal Types
-- ========================================================================
-- 
-- This file contains reusable macros and utility functions to work with
-- Vortex files in DuckDB. Temporal types (DATE, TIMESTAMP, INTERVAL) are
-- stored as numbers in Vortex for efficiency and need conversion.
--
-- USAGE:
--   1. Load this file in DuckDB: .read examples/duckdb_helpers.sql
--   2. Use the macros in your queries
--   3. Or create views for your specific tables
--
-- Author: William Gacquer
-- License: EUPL-1.2
-- ========================================================================

-- ========================================================================
-- TEMPORAL CONVERSION MACROS
-- ========================================================================

-- Convert Vortex DATE (I32 days since epoch) to DuckDB DATE
CREATE OR REPLACE MACRO vortex_to_date(days) AS (
    DATE '1970-01-01' + INTERVAL (days) DAYS
);

-- Convert Vortex TIMESTAMP (I64 microseconds since epoch) to DuckDB TIMESTAMP
CREATE OR REPLACE MACRO vortex_to_timestamp(micros) AS (
    to_timestamp(micros / 1000000.0)
);

-- Convert Vortex TIMESTAMP WITH TIME ZONE (I64 UTC microseconds) to TIMESTAMPTZ
CREATE OR REPLACE MACRO vortex_to_timestamptz(micros) AS (
    to_timestamp(micros / 1000000.0)
);

-- ========================================================================
-- INTERVAL CONVERSION MACROS
-- ========================================================================

-- Convert Vortex INTERVAL DAY TO SECOND (I64 microseconds) to DuckDB INTERVAL
CREATE OR REPLACE MACRO vortex_to_interval_ds(micros) AS (
    INTERVAL (micros / 1000000.0) SECONDS
);

-- Convert Vortex INTERVAL YEAR TO MONTH (I32 months) to DuckDB INTERVAL
CREATE OR REPLACE MACRO vortex_to_interval_ym(months) AS (
    INTERVAL (months) MONTHS
);

-- Extract days from INTERVAL DAY TO SECOND microseconds
CREATE OR REPLACE MACRO interval_ds_days(micros) AS (
    CAST(micros / 86400000000.0 AS INTEGER)
);

-- Extract hours from INTERVAL DAY TO SECOND microseconds
CREATE OR REPLACE MACRO interval_ds_hours(micros) AS (
    CAST((micros % 86400000000) / 3600000000.0 AS INTEGER)
);

-- Extract minutes from INTERVAL DAY TO SECOND microseconds
CREATE OR REPLACE MACRO interval_ds_minutes(micros) AS (
    CAST((micros % 3600000000) / 60000000.0 AS INTEGER)
);

-- Extract seconds from INTERVAL DAY TO SECOND microseconds
CREATE OR REPLACE MACRO interval_ds_seconds(micros) AS (
    (micros % 60000000) / 1000000.0
);

-- Extract years from INTERVAL YEAR TO MONTH months
CREATE OR REPLACE MACRO interval_ym_years(months) AS (
    CAST(months / 12 AS INTEGER)
);

-- Extract months from INTERVAL YEAR TO MONTH months
CREATE OR REPLACE MACRO interval_ym_months(months) AS (
    months % 12
);

-- ========================================================================
-- BINARY DATA MACROS
-- ========================================================================

-- Convert binary data to hexadecimal string
CREATE OR REPLACE MACRO vortex_to_hex(binary_data) AS (
    hex(binary_data)
);

-- Get size of binary data in bytes
CREATE OR REPLACE MACRO vortex_binary_size(binary_data) AS (
    octet_length(binary_data)
);

-- ========================================================================
-- DATE/TIMESTAMP RANGE FILTERS (OPTIMIZED)
-- ========================================================================
-- These macros convert DuckDB dates to Vortex format for efficient filtering
-- on raw values (faster than converting every row)

-- Convert DuckDB DATE to Vortex I32 days (for efficient WHERE clauses)
CREATE OR REPLACE MACRO date_to_vortex(dt) AS (
    CAST(date_diff('day', DATE '1970-01-01', dt) AS INTEGER)
);

-- Convert DuckDB TIMESTAMP to Vortex I64 microseconds (for efficient WHERE clauses)
CREATE OR REPLACE MACRO timestamp_to_vortex(ts) AS (
    CAST(epoch_ms(ts) * 1000 AS BIGINT)
);

-- ========================================================================
-- EXAMPLE VIEW TEMPLATES
-- ========================================================================
-- Copy and modify these templates for your specific tables

-- Example 1: Patient data with dates and timestamps
-- CREATE VIEW patients_readable AS
-- SELECT 
--     patient_id,
--     patient_name,
--     vortex_to_date(birth_date) AS birth_date,
--     vortex_to_timestamp(admission_ts) AS admission_timestamp,
--     vortex_to_timestamp(discharge_ts) AS discharge_timestamp,
--     vortex_to_interval_ds(discharge_ts - admission_ts) AS stay_duration,
--     diagnosis
-- FROM 'patients.vortex';

-- Example 2: Events with timestamps and timezones
-- CREATE VIEW events_readable AS
-- SELECT 
--     event_id,
--     event_type,
--     vortex_to_timestamptz(event_time) AS event_time_utc,
--     event_data
-- FROM 'events.vortex';

-- Example 3: Subscriptions with interval year to month
-- CREATE VIEW subscriptions_readable AS
-- SELECT 
--     subscription_id,
--     customer_id,
--     vortex_to_date(start_date) AS start_date,
--     vortex_to_interval_ym(duration) AS duration,
--     interval_ym_years(duration) AS duration_years,
--     interval_ym_months(duration) AS duration_months,
--     status
-- FROM 'subscriptions.vortex';

-- Example 4: Tasks with interval day to second
-- CREATE VIEW tasks_readable AS
-- SELECT 
--     task_id,
--     task_name,
--     vortex_to_interval_ds(estimated_duration) AS estimated_duration,
--     interval_ds_days(estimated_duration) AS est_days,
--     interval_ds_hours(estimated_duration) AS est_hours,
--     interval_ds_minutes(estimated_duration) AS est_minutes,
--     vortex_to_timestamp(created_at) AS created_at
-- FROM 'tasks.vortex';

-- ========================================================================
-- PERFORMANCE-OPTIMIZED QUERIES
-- ========================================================================
-- Filter on raw values, convert only for display (much faster!)

-- Example: Find patients born after 2000-01-01
-- Fast approach (filter raw, convert for display):
-- SELECT 
--     patient_id,
--     vortex_to_date(birth_date) AS birth_date
-- FROM 'patients.vortex'
-- WHERE birth_date > date_to_vortex('2000-01-01')  -- Filter on raw I32 value
-- LIMIT 100;

-- Slow approach (convert then filter):
-- SELECT 
--     patient_id,
--     vortex_to_date(birth_date) AS birth_date
-- FROM 'patients.vortex'
-- WHERE vortex_to_date(birth_date) > '2000-01-01'  -- Converts every row!
-- LIMIT 100;

-- ========================================================================
-- EPOCH REFERENCE VALUES
-- ========================================================================
-- Common dates as Vortex I32 values for quick reference in WHERE clauses

-- Milestone dates:
-- 1970-01-01 = 0 (epoch)
-- 1980-01-01 = 3653
-- 1990-01-01 = 7305
-- 2000-01-01 = 10957
-- 2010-01-01 = 14610
-- 2020-01-01 = 18262
-- 2024-01-01 = 19723
-- 2025-01-01 = 20088
-- 2026-01-01 = 20453

-- Quick filters using literal values (fastest):
-- SELECT * FROM 'patients.vortex' WHERE birth_date > 10957;  -- After 2000-01-01

-- ========================================================================
-- UTILITIES
-- ========================================================================

-- Show all macros defined in this file
CREATE OR REPLACE MACRO show_vortex_macros() AS (
    SELECT function_name, description
    FROM duckdb_functions()
    WHERE function_name LIKE 'vortex_%'
       OR function_name LIKE 'interval_%'
       OR function_name LIKE 'date_to_vortex'
       OR function_name LIKE 'timestamp_to_vortex'
    ORDER BY function_name
);

-- ========================================================================
-- USAGE EXAMPLES
-- ========================================================================

-- After loading this file, use the macros:

-- 1. Simple conversion in SELECT:
--    SELECT vortex_to_date(19723);  -- Returns 2024-01-01

-- 2. Convert columns in a query:
--    SELECT 
--        patient_id,
--        vortex_to_date(birth_date) AS birth_date
--    FROM 'patients.vortex';

-- 3. Use in WHERE clause for filtering:
--    SELECT * 
--    FROM 'patients.vortex'
--    WHERE birth_date BETWEEN date_to_vortex('1990-01-01') 
--                         AND date_to_vortex('2000-12-31');

-- 4. Create a view and query it naturally:
--    CREATE VIEW patients AS
--    SELECT 
--        patient_id,
--        vortex_to_date(birth_date) AS birth_date,
--        vortex_to_timestamp(admission_ts) AS admission_ts
--    FROM 'patients.vortex';
--    
--    SELECT * FROM patients WHERE birth_date > '2000-01-01';

-- 5. List all available macros:
--    SELECT show_vortex_macros();

-- ========================================================================
-- READY TO USE
-- ========================================================================
SELECT 'DuckDB Vortex helper macros loaded successfully!' AS status;
SELECT 'Use show_vortex_macros() to see all available functions' AS info;
