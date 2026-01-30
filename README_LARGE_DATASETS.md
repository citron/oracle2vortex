# Handling Large Datasets (Millions to Billions of Rows)

## Quick Answer

✅ **For < 100K rows**: Use as-is, works perfectly  
✅ **For 100K - 100M rows**: Use `--auto-batch-rows` parameter (recommended: 50000)  
✅ **For 100M - 1B rows**: Use `--auto-batch-rows` with larger batches (500000+)  
✅ **For > 1B rows**: Use `--auto-batch-rows` + partitioning/date ranges + parallel processing

## The Limitation

SQLcl's JSON format returns one giant JSON object **per query**:
```json
{"results":[{"columns":[...],"items":[{},{},{},...]}]}
```

All rows are in the `items` array → entire result loaded into memory.

**Memory required per query** = ~2× JSON output size (JSON + Vortex buffers)

## ✅ Solution 1: Auto-Batching (Easiest)

### Use `--auto-batch-rows` parameter

This automatically splits your query into batches:

```bash
# Process millions of rows in batches of 50,000
oracle2vortex \
  --sql-file query.sql \
  --output data.vortex \
  --host myhost \
  --port 1521 \
  --user myuser \
  --password mypass \
  --sid MYSID \
  --sqlcl-path /opt/oracle/sqlcl/bin/sql \
  --auto-batch-rows 50000
```

**How it works:**
1. Wraps your query with `OFFSET/FETCH` automatically
2. Executes SQLcl multiple times (one per batch)
3. Accumulates all records in memory
4. Writes single Vortex file
5. Stops when partial batch received

**Advantages:**
- ✅ Simple: just add one parameter
- ✅ Single output file
- ✅ Automatic stop detection
- ✅ Progress logging

**Limitations:**
- Requires Oracle 12c+ (OFFSET/FETCH syntax)
- Query must NOT already contain OFFSET/FETCH or ROWNUM
- Still accumulates ALL records in memory before writing (future: incremental write)

### Examples

#### 1 million rows
```bash
oracle2vortex -f query.sql -o output.vortex ... --auto-batch-rows 50000
# 20 batches × 50K rows = 1M rows
# Memory: ~100 MB per batch
# Time: ~3-5 minutes
```

#### 100 million rows
```bash
oracle2vortex -f query.sql -o output.vortex ... --auto-batch-rows 500000
# 200 batches × 500K rows = 100M rows  
# Memory: ~1 GB per batch
# Time: ~1-2 hours
```

#### 1 billion rows
```bash
oracle2vortex -f query.sql -o output.vortex ... --auto-batch-rows 1000000
# 1000 batches × 1M rows = 1B rows
# Memory: ~2 GB per batch
# Time: ~10-20 hours
```

### Choosing Batch Size

| Dataset Size | `--auto-batch-rows` | Memory/Batch | Batches (est.) |
|--------------|---------------------|--------------|----------------|
| 100K - 1M | 10,000 - 50,000 | 20-100 MB | 2 - 100 |
| 1M - 10M | 50,000 - 100,000 | 100-200 MB | 10 - 200 |
| 10M - 100M | 100,000 - 500,000 | 200MB - 1GB | 20 - 1000 |
| 100M - 1B | 500,000 - 1,000,000 | 1-2 GB | 100 - 2000 |
| > 1B | See Solution 2 | | |

**Rule of thumb:** Batch size × row size × 2 = memory per batch  
Example: 50,000 rows × 1 KB × 2 = 100 MB

## Solution 2: Manual Batching (Multiple Files)

If auto-batching doesn't fit your needs, manually split queries:

```bash
#!/bin/bash
# Process 1 billion rows in batches of 50K

BATCH_SIZE=50000
TOTAL_ROWS=1000000000

for ((offset=0; offset<$TOTAL_ROWS; offset+=$BATCH_SIZE)); do
  echo "Processing rows $offset to $((offset+BATCH_SIZE))..."
  
  cat > batch.sql <<SQL
SELECT * FROM my_large_table 
ORDER BY id  -- Important: consistent ordering
OFFSET $offset ROWS 
FETCH NEXT $BATCH_SIZE ROWS ONLY;
SQL

  ./target/release/oracle2vortex \
    --sql-file batch.sql \
    --output "output_batch_${offset}.vortex" \
    --host myhost \
    --port 1521 \
    --user myuser \
    --password mypass \
    --sid MYSID \
    --sqlcl-path /opt/oracle/sqlcl/bin/sql
    
  rm batch.sql
done

echo "Done! Created $((TOTAL_ROWS / BATCH_SIZE)) Vortex files"
```

### Oracle 11g and Earlier

Use ROWNUM:

```sql
-- Batch 1: rows 1-50000
SELECT * FROM (
  SELECT a.*, ROWNUM rnum FROM (
    SELECT * FROM my_large_table ORDER BY id
  ) a WHERE ROWNUM <= 50000
) WHERE rnum >= 1;

-- Batch 2: rows 50001-100000
SELECT * FROM (
  SELECT a.*, ROWNUM rnum FROM (
    SELECT * FROM my_large_table ORDER BY id
  ) a WHERE ROWNUM <= 100000
) WHERE rnum >= 50001;
```

## Performance Tips

### 1. Use Indexes
```sql
CREATE INDEX idx_mytable_id ON my_large_table(id);
```

### 2. Adjust Batch Size

- **More memory available**: Increase batch size (100K - 500K rows)
- **Limited memory**: Decrease batch size (10K - 25K rows)
- **Default 50K**: Good balance for most cases

### 3. Parallel Processing

Split by natural partitions:

```bash
# Terminal 1
oracle2vortex --sql-file "SELECT * FROM sales WHERE year=2020" ...

# Terminal 2  
oracle2vortex --sql-file "SELECT * FROM sales WHERE year=2021" ...

# Terminal 3
oracle2vortex --sql-file "SELECT * FROM sales WHERE year=2022" ...
```

### 4. Monitor Memory

```bash
/usr/bin/time -v ./target/release/oracle2vortex ... 2>&1 | grep "Maximum resident set size"
```

Expected: ~1-2GB for 50K row batch

## Why JSON Format?

### ✅ Type Preservation

JSON preserves Oracle column types:

| Oracle Type | JSON Type | Vortex Type | CSV Loses? |
|-------------|-----------|-------------|------------|
| NUMBER(10,0) | number (int) | I64 | ✅ → becomes string |
| NUMBER(10,2) | number (float) | F64 | ✅ → becomes string |
| VARCHAR2 | string | Utf8 | ❌ preserved |
| DATE | string (ISO) | Utf8 | ❌ preserved |
| BOOLEAN | boolean | Bool | ✅ → becomes string |
| NULL | null | Nullable | ⚠️ → becomes empty string |

### ❌ CSV Issues

1. **Type loss**: All values become strings
2. **Decimal separator**: French locale → `108,8` breaks CSV parsing
3. **NULL ambiguity**: Empty string vs actual NULL

## Example: 100 Million Rows

Table: `transactions` (500 columns, ~10KB per row)

### Memory Requirements

**Without batching:**
```
100M rows × 10KB = 1TB JSON
Memory needed: ~2TB ❌ IMPOSSIBLE on most machines
```

**With batching (50K rows):**
```
50K rows × 10KB = 500MB JSON
Memory needed: ~1GB per batch ✅
Total batches: 100M / 50K = 2,000 batches
Total time: ~2,000 × 5 sec = ~3 hours
```

### Script

```bash
#!/bin/bash
BATCH_SIZE=50000
TABLE="transactions"

# Get total row count
TOTAL=$(sqlplus -S user/pass@db <<< "SET PAGESIZE 0
SELECT COUNT(*) FROM $TABLE;
EXIT")

echo "Total rows: $TOTAL"
echo "Batches: $((TOTAL / BATCH_SIZE))"

for ((offset=0; offset<$TOTAL; offset+=$BATCH_SIZE)); do
  batch_num=$((offset / BATCH_SIZE + 1))
  echo "[$batch_num] Processing rows $offset..."
  
  cat > /tmp/batch.sql <<SQL
SELECT * FROM $TABLE 
ORDER BY transaction_id 
OFFSET $offset ROWS 
FETCH NEXT $BATCH_SIZE ROWS ONLY;
SQL

  ./target/release/oracle2vortex \
    --sql-file /tmp/batch.sql \
    --output "transactions_part_$(printf "%05d" $batch_num).vortex" \
    --host proddb.company.com \
    --port 1521 \
    --user readonly_user \
    --password "$DB_PASSWORD" \
    --sid PROD \
    --sqlcl-path /opt/oracle/sqlcl/bin/sql
    
  if [ $? -ne 0 ]; then
    echo "ERROR at batch $batch_num"
    exit 1
  fi
done

rm /tmp/batch.sql
echo "SUCCESS: Created $((TOTAL / BATCH_SIZE)) Vortex files"
```

## Future Feature: Auto-Batching

Planned enhancement:

```bash
# Future syntax (not yet implemented)
oracle2vortex \
  --sql-file "SELECT * FROM huge_table" \
  --output combined.vortex \
  --max-rows-per-batch 50000 \
  --auto-merge \
  ...
```

Would automatically:
1. Wrap query with OFFSET/FETCH
2. Execute in loop
3. Append results to single Vortex file

Current status: ⬜ Not implemented (use manual batching for now)

## Questions?

See `BATCH_PROCESSING.md` for technical details.
