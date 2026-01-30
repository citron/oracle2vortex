# Examples - oracle2vortex

This directory contains example SQL files for testing oracle2vortex.

## sample_query.sql

A basic query example that fetches employee data from the HR schema (standard Oracle sample schema).

### Usage Example

```bash
../target/release/oracle2vortex \
  -f sample_query.sql \
  -o employees.vortex \
  --host localhost \
  --port 1521 \
  -u hr \
  -p hr_password \
  --sid XEPDB1 \
  --sqlcl-path /opt/oracle/sqlcl/bin/sql

# Verify the output
vx browse employees.vortex
```

## Creating Your Own Queries

### Guidelines

1. **Use SELECT statements only** - No PL/SQL blocks, no DDL
2. **Return tabular data** - Standard column/row result set
3. **Check privileges** - User must have SELECT on all referenced tables
4. **Limit for testing** - Use `WHERE ROWNUM <= 100` for initial tests

### Example Queries

**Simple table export**:
```sql
SELECT * FROM employees WHERE department_id = 50;
```

**With column selection**:
```sql
SELECT 
    employee_id,
    first_name || ' ' || last_name AS full_name,
    salary,
    hire_date
FROM employees
WHERE hire_date >= DATE '2020-01-01';
```

**With aggregation**:
```sql
SELECT 
    department_id,
    COUNT(*) AS employee_count,
    AVG(salary) AS avg_salary,
    MAX(hire_date) AS latest_hire
FROM employees
GROUP BY department_id
ORDER BY department_id;
```

**Limited rows for testing**:
```sql
SELECT * FROM large_table WHERE ROWNUM <= 1000;
```

## Schema Inference

The Vortex schema is **automatically inferred from the first record**.

### Type Mapping

| Oracle SQL Type | JSON Type | Vortex Type |
|----------------|-----------|-------------|
| NUMBER (int) | number | I64 |
| NUMBER (decimal) | number | F64 |
| VARCHAR2, CHAR, CLOB | string | Utf8 |
| DATE, TIMESTAMP | string | Utf8 (ISO format) |
| BOOLEAN (23ai+) | boolean | Bool |
| NULL | null | Utf8 (nullable) |

### Tips for Consistent Schema

1. **Cast ambiguous types**:
   ```sql
   SELECT 
       CAST(numeric_col AS NUMBER(10,0)) AS int_col,
       CAST(text_col AS VARCHAR2(100)) AS str_col
   FROM my_table;
   ```

2. **Handle NULLs explicitly**:
   ```sql
   SELECT 
       COALESCE(nullable_col, 0) AS col_with_default
   FROM my_table;
   ```

3. **Format dates consistently**:
   ```sql
   SELECT 
       TO_CHAR(date_col, 'YYYY-MM-DD"T"HH24:MI:SS') AS iso_date
   FROM my_table;
   ```

## Performance Considerations

- **Large datasets**: Currently buffers all records; plan for ~2x dataset size in memory
- **Wide tables**: 417 columns tested successfully (1.3 MB for 100 rows)
- **Recommended batch size**: 10,000 - 1,000,000 rows depending on column count

## Validation

After running oracle2vortex, verify the output:

```bash
# Install vx tool
cargo install vortex-vx

# Browse the file interactively
vx browse output.vortex

# Show file metadata
vx info output.vortex

# Show statistics
vx stats output.vortex
```

## Security Notes

⚠️ **Never commit files with credentials to git!**

Use `tests_local/` directory for queries with real connection info:
```bash
# Create test query (gitignored)
echo "SELECT * FROM sensitive_table WHERE ROWNUM <= 10;" > ../tests_local/my_test.sql

# Run without exposing credentials in history
read -s ORACLE_PASS
../target/release/oracle2vortex \
  -f ../tests_local/my_test.sql \
  -o ../tests_local/output.vortex \
  --host myhost.example.com \
  --port 1521 \
  -u myuser \
  -p "$ORACLE_PASS" \
  --sid PROD
```
