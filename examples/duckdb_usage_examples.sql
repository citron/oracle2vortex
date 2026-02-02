-- ========================================================================
-- DuckDB Usage Examples for Vortex Files
-- ========================================================================
--
-- This file demonstrates practical examples of querying Vortex files
-- exported from Oracle using oracle2vortex.
--
-- PREREQUISITES:
--   1. Load helper macros: .read examples/duckdb_helpers.sql
--   2. Have Vortex files from oracle2vortex
--
-- Author: William Gacquer
-- License: EUPL-1.2
-- ========================================================================

-- ========================================================================
-- EXAMPLE 1: Employee Data with Hire Dates
-- ========================================================================

-- Assuming you exported: SELECT * FROM employees
-- Columns: employee_id, first_name, last_name, hire_date, salary

-- 1a. View raw data (dates appear as numbers)
SELECT * FROM 'employees.vortex' LIMIT 5;

-- 1b. Convert hire_date to readable format
SELECT 
    employee_id,
    first_name,
    last_name,
    vortex_to_date(hire_date) AS hire_date,
    salary
FROM 'employees.vortex'
LIMIT 5;

-- 1c. Create a readable view
CREATE OR REPLACE VIEW employees AS
SELECT 
    employee_id,
    first_name,
    last_name,
    vortex_to_date(hire_date) AS hire_date,
    salary
FROM 'employees.vortex';

-- 1d. Query the view naturally
SELECT * FROM employees WHERE hire_date > '2020-01-01';

-- 1e. Performance-optimized filter (filter raw values, convert for display)
SELECT 
    employee_id,
    first_name,
    vortex_to_date(hire_date) AS hire_date
FROM 'employees.vortex'
WHERE hire_date > date_to_vortex('2020-01-01')  -- Fast: filter on raw I32
ORDER BY hire_date DESC;

-- ========================================================================
-- EXAMPLE 2: Patient Records with Multiple Temporal Columns
-- ========================================================================

-- Assuming export from: SELECT patient_id, patient_name, birth_date, 
--                              admission_ts, discharge_ts, diagnosis
-- FROM patient_records

-- 2a. Convert all temporal columns
SELECT 
    patient_id,
    patient_name,
    vortex_to_date(birth_date) AS birth_date,
    vortex_to_timestamp(admission_ts) AS admission_timestamp,
    vortex_to_timestamp(discharge_ts) AS discharge_timestamp,
    diagnosis
FROM 'patient_records.vortex'
LIMIT 10;

-- 2b. Calculate length of stay
SELECT 
    patient_id,
    patient_name,
    vortex_to_timestamp(admission_ts) AS admission,
    vortex_to_timestamp(discharge_ts) AS discharge,
    vortex_to_interval_ds(discharge_ts - admission_ts) AS length_of_stay,
    interval_ds_days(discharge_ts - admission_ts) AS days_hospitalized
FROM 'patient_records.vortex'
WHERE discharge_ts IS NOT NULL
ORDER BY length_of_stay DESC
LIMIT 20;

-- 2c. Age calculation at admission
SELECT 
    patient_id,
    patient_name,
    vortex_to_date(birth_date) AS birth_date,
    vortex_to_timestamp(admission_ts) AS admission,
    CAST(date_diff('year', 
        vortex_to_date(birth_date), 
        CAST(vortex_to_timestamp(admission_ts) AS DATE)
    ) AS INTEGER) AS age_at_admission
FROM 'patient_records.vortex'
LIMIT 10;

-- 2d. Create comprehensive view
CREATE OR REPLACE VIEW patient_records AS
SELECT 
    patient_id,
    patient_name,
    vortex_to_date(birth_date) AS birth_date,
    vortex_to_timestamp(admission_ts) AS admission_timestamp,
    vortex_to_timestamp(discharge_ts) AS discharge_timestamp,
    vortex_to_interval_ds(discharge_ts - admission_ts) AS stay_duration,
    interval_ds_days(discharge_ts - admission_ts) AS days_hospitalized,
    diagnosis
FROM 'patient_records.vortex';

-- Query the view
SELECT * 
FROM patient_records 
WHERE birth_date > '1980-01-01' 
  AND days_hospitalized > 7
ORDER BY admission_timestamp DESC;

-- ========================================================================
-- EXAMPLE 3: Event Logs with Timestamps and Time Zones
-- ========================================================================

-- Assuming export from: SELECT event_id, event_type, event_time, 
--                              event_data, user_id
-- FROM event_logs
-- WHERE event_time is TIMESTAMP WITH TIME ZONE (stored as UTC in Vortex)

-- 3a. View events with converted timestamps
SELECT 
    event_id,
    event_type,
    vortex_to_timestamptz(event_time) AS event_time_utc,
    user_id
FROM 'event_logs.vortex'
ORDER BY event_time DESC
LIMIT 50;

-- 3b. Events in last 24 hours
SELECT 
    event_id,
    event_type,
    vortex_to_timestamptz(event_time) AS event_time_utc,
    user_id
FROM 'event_logs.vortex'
WHERE vortex_to_timestamptz(event_time) > current_timestamp - INTERVAL '24 hours'
ORDER BY event_time DESC;

-- 3c. Group by hour
SELECT 
    date_trunc('hour', vortex_to_timestamptz(event_time)) AS hour,
    event_type,
    count(*) AS event_count
FROM 'event_logs.vortex'
WHERE vortex_to_timestamptz(event_time) > current_timestamp - INTERVAL '7 days'
GROUP BY hour, event_type
ORDER BY hour DESC, event_count DESC;

-- ========================================================================
-- EXAMPLE 4: Subscriptions with INTERVAL YEAR TO MONTH
-- ========================================================================

-- Assuming export from: SELECT subscription_id, customer_id, start_date,
--                              subscription_duration, status
-- FROM subscriptions
-- WHERE subscription_duration is INTERVAL YEAR TO MONTH

-- 4a. View subscriptions with duration
SELECT 
    subscription_id,
    customer_id,
    vortex_to_date(start_date) AS start_date,
    vortex_to_interval_ym(subscription_duration) AS duration,
    interval_ym_years(subscription_duration) AS years,
    interval_ym_months(subscription_duration) AS months,
    status
FROM 'subscriptions.vortex'
LIMIT 10;

-- 4b. Calculate end date
SELECT 
    subscription_id,
    customer_id,
    vortex_to_date(start_date) AS start_date,
    vortex_to_date(start_date) + vortex_to_interval_ym(subscription_duration) AS end_date,
    interval_ym_years(subscription_duration) AS duration_years,
    status
FROM 'subscriptions.vortex'
WHERE status = 'ACTIVE';

-- 4c. Find expiring subscriptions (next 30 days)
SELECT 
    subscription_id,
    customer_id,
    vortex_to_date(start_date) AS start_date,
    vortex_to_date(start_date) + vortex_to_interval_ym(subscription_duration) AS end_date
FROM 'subscriptions.vortex'
WHERE status = 'ACTIVE'
  AND vortex_to_date(start_date) + vortex_to_interval_ym(subscription_duration) 
      BETWEEN current_date AND current_date + INTERVAL '30 days'
ORDER BY end_date;

-- ========================================================================
-- EXAMPLE 5: Tasks with INTERVAL DAY TO SECOND
-- ========================================================================

-- Assuming export from: SELECT task_id, task_name, estimated_duration,
--                              actual_duration, created_at
-- FROM tasks
-- WHERE estimated_duration is INTERVAL DAY TO SECOND

-- 5a. View tasks with durations
SELECT 
    task_id,
    task_name,
    vortex_to_interval_ds(estimated_duration) AS estimated,
    interval_ds_days(estimated_duration) AS est_days,
    interval_ds_hours(estimated_duration) AS est_hours,
    interval_ds_minutes(estimated_duration) AS est_minutes,
    vortex_to_timestamp(created_at) AS created_at
FROM 'tasks.vortex'
LIMIT 10;

-- 5b. Compare estimated vs actual duration
SELECT 
    task_id,
    task_name,
    vortex_to_interval_ds(estimated_duration) AS estimated,
    vortex_to_interval_ds(actual_duration) AS actual,
    vortex_to_interval_ds(actual_duration - estimated_duration) AS variance,
    CASE 
        WHEN actual_duration > estimated_duration THEN 'OVER'
        WHEN actual_duration < estimated_duration THEN 'UNDER'
        ELSE 'ON TIME'
    END AS status
FROM 'tasks.vortex'
WHERE actual_duration IS NOT NULL
ORDER BY ABS(actual_duration - estimated_duration) DESC
LIMIT 20;

-- ========================================================================
-- EXAMPLE 6: Binary Data (RAW/BLOB)
-- ========================================================================

-- Assuming export from: SELECT file_id, file_name, file_content, file_size
-- FROM files
-- WHERE file_content is RAW or BLOB

-- 6a. View file metadata
SELECT 
    file_id,
    file_name,
    vortex_binary_size(file_content) AS size_bytes,
    vortex_binary_size(file_content) / 1024.0 AS size_kb
FROM 'files.vortex'
ORDER BY size_bytes DESC
LIMIT 10;

-- 6b. View first 32 bytes as hex
SELECT 
    file_id,
    file_name,
    substring(vortex_to_hex(file_content), 1, 64) AS first_32_bytes_hex
FROM 'files.vortex'
LIMIT 5;

-- 6c. Find files by size
SELECT 
    file_id,
    file_name,
    vortex_binary_size(file_content) / 1024.0 AS size_kb
FROM 'files.vortex'
WHERE vortex_binary_size(file_content) BETWEEN 1024 AND 10240  -- 1KB to 10KB
ORDER BY size_kb DESC;

-- ========================================================================
-- EXAMPLE 7: Complex Multi-Table Analysis
-- ========================================================================

-- Assuming multiple Vortex files from different tables

-- 7a. Create views for all tables
CREATE OR REPLACE VIEW employees AS
SELECT 
    employee_id,
    first_name,
    last_name,
    vortex_to_date(hire_date) AS hire_date,
    department_id,
    salary
FROM 'employees.vortex';

CREATE OR REPLACE VIEW departments AS
SELECT 
    department_id,
    department_name,
    manager_id
FROM 'departments.vortex';

-- 7b. Join tables
SELECT 
    e.employee_id,
    e.first_name,
    e.last_name,
    e.hire_date,
    d.department_name,
    e.salary
FROM employees e
JOIN departments d ON e.department_id = d.department_id
WHERE e.hire_date > '2020-01-01'
ORDER BY e.hire_date DESC;

-- 7c. Department statistics
SELECT 
    d.department_name,
    count(*) AS employee_count,
    avg(e.salary) AS avg_salary,
    min(e.hire_date) AS earliest_hire,
    max(e.hire_date) AS latest_hire
FROM employees e
JOIN departments d ON e.department_id = d.department_id
GROUP BY d.department_name
ORDER BY employee_count DESC;

-- ========================================================================
-- EXAMPLE 8: Export Results to Standard Formats
-- ========================================================================

-- 8a. Export to CSV with readable dates
COPY (
    SELECT 
        employee_id,
        first_name,
        last_name,
        vortex_to_date(hire_date) AS hire_date,
        salary
    FROM 'employees.vortex'
) TO 'employees_readable.csv' (HEADER, DELIMITER ',');

-- 8b. Export to Parquet with converted types
COPY (
    SELECT 
        patient_id,
        patient_name,
        vortex_to_date(birth_date) AS birth_date,
        vortex_to_timestamp(admission_ts) AS admission_timestamp,
        vortex_to_timestamp(discharge_ts) AS discharge_timestamp,
        diagnosis
    FROM 'patient_records.vortex'
) TO 'patient_records_readable.parquet';

-- 8c. Export to JSON
COPY (
    SELECT 
        event_id,
        event_type,
        vortex_to_timestamptz(event_time) AS event_time,
        user_id
    FROM 'event_logs.vortex'
    WHERE vortex_to_timestamptz(event_time) > current_timestamp - INTERVAL '1 day'
) TO 'recent_events.json';

-- ========================================================================
-- EXAMPLE 9: Performance Comparison
-- ========================================================================

-- 9a. SLOW: Convert then filter (processes all rows)
EXPLAIN ANALYZE
SELECT 
    patient_id,
    vortex_to_date(birth_date) AS birth_date
FROM 'patient_records.vortex'
WHERE vortex_to_date(birth_date) > '2000-01-01'  -- Converts EVERY row first
LIMIT 100;

-- 9b. FAST: Filter raw values then convert (filters first)
EXPLAIN ANALYZE
SELECT 
    patient_id,
    vortex_to_date(birth_date) AS birth_date
FROM 'patient_records.vortex'
WHERE birth_date > date_to_vortex('2000-01-01')  -- Filters on I32, very fast
LIMIT 100;

-- 9c. FASTEST: Use literal epoch value (no function call overhead)
EXPLAIN ANALYZE
SELECT 
    patient_id,
    vortex_to_date(birth_date) AS birth_date
FROM 'patient_records.vortex'
WHERE birth_date > 10957  -- Literal value for 2000-01-01
LIMIT 100;

-- ========================================================================
-- DONE
-- ========================================================================
SELECT 'All examples completed!' AS status;
SELECT 'Modify these templates for your specific use cases' AS tip;
