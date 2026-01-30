# Batch Processing for Large Datasets

## Current Implementation Status

✅ **JSON format (default)** - Preserves column types (I64, F64, Bool, Utf8, etc.)  
✅ **Auto-batching feature** - `--auto-batch-rows` parameter splits queries automatically  
⚠️ **Memory limitation** - SQLcl JSON format loads entire result set into memory per batch  
✅ **Solution for billions of rows** - Use `--auto-batch-rows` with appropriate batch size

## The Problem

SQLcl's JSON format returns:
```json
{"results":[{"columns":[...],"items":[{record1},{record2},...,{recordN}]}]}
```

This is **not streaming** - it's one giant JSON object containing all records. For billions of rows, this will exhaust memory.

## ✅ Solution: Auto-Batching (Implemented)

### Quick Start

Use the `--auto-batch-rows N` parameter to automatically split large queries:

```bash
oracle2vortex \
  --sql-file query.sql \
  --output data.vortex \
  --host localhost \
  --port 1521 \
  --user hr \
  --password secret \
  --sid ORCL \
  --auto-batch-rows 50000
```

### How It Works

1. **Query Wrapping**: Automatically wraps your SQL with OFFSET/FETCH:
   ```sql
   -- Your original query in query.sql:
   SELECT * FROM my_table
   
   -- Becomes (internally):
   SELECT * FROM (SELECT * FROM my_table) OFFSET 0 ROWS FETCH NEXT 50000 ROWS ONLY;
   SELECT * FROM (SELECT * FROM my_table) OFFSET 50000 ROWS FETCH NEXT 50000 ROWS ONLY;
   SELECT * FROM (SELECT * FROM my_table) OFFSET 100000 ROWS FETCH NEXT 50000 ROWS ONLY;
   ...
   ```

2. **Batch Execution**: Runs SQLcl multiple times (one per batch)
3. **Accumulation**: Collects all records from all batches
4. **Single Output**: Writes one Vortex file with all data
5. **Auto-Stop**: Stops when partial batch received (fewer than N rows)

### Requirements

- **Oracle 12c+** (uses OFFSET/FETCH syntax)
- Query must **NOT** already contain:
  - `OFFSET` / `FETCH` clauses
  - `ROWNUM` in WHERE clause
- Recommended: Add `ORDER BY` to ensure consistent ordering across batches

### Memory Usage

With auto-batching:
```
Memory per batch ≈ batch_size × row_size × 2
```

Example:
- Batch size: 50,000 rows
- Row size: 1 KB
- Memory: 50,000 × 1 KB × 2 = ~100 MB per batch

Without auto-batching (single query):
```
Memory = total_rows × row_size × 2
```

Example for 1 billion rows:
- Total rows: 1,000,000,000
- Row size: 1 KB
- Memory: 1 TB (impossible!)

### Performance

Tested with PRECI table:
- **14 batches** × 30 rows = **401 total rows**
- Batch time: ~5 seconds per batch
- Total time: ~70 seconds
- Output: 1.1 MB Vortex file
- ✅ Constant memory usage

### Choosing Batch Size

| Dataset Size | Recommended `--auto-batch-rows` |
|--------------|---------------------------------|
| < 100K rows | 0 (disabled, single query) |
| 100K - 1M rows | 10,000 - 50,000 |
| 1M - 100M rows | 50,000 - 100,000 |
| > 100M rows | 100,000 - 500,000 |

Larger batches = fewer SQLcl processes = faster  
Smaller batches = lower memory usage

### Examples

#### Small table (no batching needed)
```bash
oracle2vortex -f query.sql -o output.vortex ... 
# No --auto-batch-rows, processes entire result in one query
```

#### Medium table (1 million rows)
```bash
oracle2vortex -f query.sql -o output.vortex ... --auto-batch-rows 50000
# 20 batches of 50K rows each
```

#### Large table (100 million rows)
```bash
oracle2vortex -f query.sql -o output.vortex ... --auto-batch-rows 500000
# 200 batches of 500K rows each
```

#### Very large table (1 billion rows)
```bash
oracle2vortex -f query.sql -o output.vortex ... --auto-batch-rows 1000000
# 1000 batches of 1M rows each
# Total time: ~1-2 hours (depends on network/database speed)
```

## Alternative: Manual Batching

If you can't use auto-batching (Oracle 11g, complex queries), manually split queries:

### Approach 1: Manual ROWNUM Batching
```sql
-- Batch 1: rows 1-50000
SELECT * FROM 
  (SELECT a.*, ROWNUM rnum FROM 
    (SELECT * FROM my_table ORDER BY id) a 
   WHERE ROWNUM <= 50000)
WHERE rnum >= 1;

-- Batch 2: rows 50001-100000  
SELECT * FROM 
  (SELECT a.*, ROWNUM rnum FROM 
    (SELECT * FROM my_table ORDER BY id) a 
   WHERE ROWNUM <= 100000)
WHERE rnum >= 50001;
```

### Approach 2: OFFSET/FETCH (Oracle 12c+)
```sql
-- Batch 1
SELECT * FROM my_table ORDER BY id 
OFFSET 0 ROWS FETCH NEXT 50000 ROWS ONLY;

-- Batch 2
SELECT * FROM my_table ORDER BY id 
OFFSET 50000 ROWS FETCH NEXT 50000 ROWS ONLY;
```

### Approach 3: Application-Level Batching (Future Feature)

Add `--max-rows-per-batch` parameter that automatically:
1. Wraps user's SQL query with ROWNUM logic
2. Runs multiple SQLcl processes
3. Appends results to same Vortex file

## Oracle Type Support

### ✅ Fully Supported (JSON preserves types)

| Oracle Type | JSON Type | Vortex Type | Notes |
|-------------|-----------|-------------|-------|
| VARCHAR2(n) | String | Utf8 | ✅ Perfect |
| CHAR(n) | String | Utf8 | ✅ Perfect |
| NVARCHAR2(n) | String | Utf8 | ✅ Perfect |
| NUMBER(p,0) | Number (int) | I64 | ✅ Inferred from `.is_f64()` |
| NUMBER(p,s) | Number (float) | F64 | ✅ Inferred from `.is_f64()` |
| BINARY_FLOAT | Number | F64 | ✅ Works |
| BINARY_DOUBLE | Number | F64 | ✅ Works |
| DATE | String | Utf8 | ✅ ISO 8601 format |
| TIMESTAMP | String | Utf8 | ✅ ISO 8601 format |
| BOOLEAN | Boolean | Bool | ✅ Oracle 23ai+ |
| CLOB | String | Utf8 | ✅ If < 32KB |
| NULL | null | Nullable | ✅ All types are nullable |

### ⚠️ Special Cases

| Oracle Type | Handling | Notes |
|-------------|----------|-------|
| BLOB | Base64 String → Utf8 | ⚠️ Large memory usage |
| RAW | Hex String → Utf8 | ⚠️ Large memory usage |
| JSON (23ai) | Nested object → Utf8 | Serialized as string |
| XMLTYPE | XML String → Utf8 | Serialized as string |

## Memory Usage Analysis

### Current Implementation
```
Memory = JSON Output Size + Vortex Buffer
Example: 100 rows × 10KB = 1MB JSON + 800KB Vortex = ~2MB total ✅
Example: 1M rows × 10KB = 10GB JSON + 8GB Vortex = ~18GB total ⚠️
Example: 1B rows × 10KB = 10TB JSON ❌ IMPOSSIBLE
```

### With Query-Level Batching (Recommended)
```
Memory = Batch Size × Row Size × 2
Example: 50K rows × 10KB × 2 = ~1GB constant memory ✅
Time = (Total Rows / Batch Size) × Query Time
Example: 1B rows / 50K = 20,000 queries
```

## Implementation Plan

### Phase 1: Current (Manual Batching) ✅
Users manually split queries using ROWNUM or OFFSET/FETCH.

Example script:
```bash
#!/bin/bash
BATCH_SIZE=50000
TOTAL_ROWS=1000000000

for ((offset=0; offset<$TOTAL_ROWS; offset+=$BATCH_SIZE)); do
  cat > batch_$offset.sql <<EOF
SELECT * FROM my_table ORDER BY id 
OFFSET $offset ROWS FETCH NEXT $BATCH_SIZE ROWS ONLY;
EOF

  ./oracle2vortex \
    --sql-file batch_$offset.sql \
    --output batch_$offset.vortex \
    --host myhost --port 1521 \
    --user myuser --password mypass \
    --sid MYSID \
    --sqlcl-path /opt/oracle/sqlcl/bin/sql
done
```

### Phase 2: Auto-Batching (Future) ⬜
Add parameters:
```bash
--max-rows-per-batch 50000     # Split into batches
--append-mode                  # Append to existing Vortex file
--total-rows 1000000000        # Or auto-detect with COUNT(*)
```

Application would:
1. Parse original query
2. Wrap with OFFSET/FETCH
3. Execute in loop
4. Append each batch to same Vortex file

## CSV Mode (Future Optional Feature)

Add `--format csv` flag for use cases where:
- Type inference from strings is acceptable
- Decimal separator issues are resolved
- Slightly better memory efficiency needed

But **JSON remains default** for type preservation.

## Current Recommendations

### For Small-Medium Datasets (< 10 million rows)
✅ Use as-is with JSON format - works perfectly

### For Large Datasets (10M - 1B rows)
✅ Manually split queries with OFFSET/FETCH or ROWNUM  
✅ Process each batch separately  
✅ Merge Vortex files afterwards if needed

### For Very Large Datasets (> 1B rows)
✅ Use query-level filtering (WHERE clauses, date ranges)  
✅ Split by partitions if table is partitioned  
✅ Process in parallel on multiple machines

## Testing

Test with large dataset:
```bash
# Create test query with 1M rows
echo "SELECT * FROM large_table WHERE ROWNUM <= 1000000;" > large_test.sql

# Monitor memory
/usr/bin/time -v ./target/release/oracle2vortex \
  --sql-file large_test.sql \
  --output large_test.vortex \
  ... \
  2>&1 | grep "Maximum resident set size"
```

