# Translation Update Notes

## Recent Changes to Include in Translations

**Last main README update:** 2026-02-02  
**Last translations update:** 2026-01-30

### Major Additions Since 2026-01-30

The following sections have been added or significantly updated in the main README and need to be translated:

#### 1. Extended Type Support Section

**New Types Added to "Supported data types" table:**

```markdown
| `INTERVAL DAY TO SECOND` | `"+02 02:30:00.123456"` | `Primitive(I64)` | I64 | Total microseconds |
| `INTERVAL YEAR TO MONTH` | `"+01-06"` | `Primitive(I32)` | I32 | Total months |
```

Under **Structured Types**:
```markdown
| `JSON` (Oracle 21c+) | `"{\"key\":\"value\"}"` | `Utf8` | VarBinArray | Validated JSON, kept as string |
| `XMLTYPE` | `"<root/>"` | `Utf8` | VarBinArray | XML as string |
```

#### 2. New Documentation References

Added after type mapping table:

```markdown
**For detailed type mapping algorithms and detection logic, see:**
- [`docs/ORACLE_TYPE_MAPPING.md`](docs/ORACLE_TYPE_MAPPING.md) - Complete technical reference with detection algorithms
- [`docs/TEMPORAL_TYPES.md`](docs/TEMPORAL_TYPES.md) - Temporal types implementation details and testing
```

#### 3. Updated Technical Details

**Temporal Types with Timezone Support** section has been enhanced with:
- More detailed timezone conversion explanation
- INTERVAL types mentioned
- Updated storage efficiency numbers

### Translation Guidelines for New Content

1. **Technical Terms** - Keep as-is:
   - `INTERVAL DAY TO SECOND`
   - `INTERVAL YEAR TO MONTH`
   - `JSON`, `XMLTYPE`
   - `Primitive(I64)`, `Primitive(I32)`
   - All data type names

2. **Format Strings** - Keep as-is:
   - `"+02 02:30:00.123456"`
   - `"+01-06"`
   - `"{\"key\":\"value\"}"`

3. **File Paths** - Keep as-is:
   - `docs/ORACLE_TYPE_MAPPING.md`
   - `docs/TEMPORAL_TYPES.md`

4. **Translatable Text**:
   - "Total microseconds" → translate
   - "Total months" → translate
   - "Validated JSON, kept as string" → translate
   - "Complete technical reference with detection algorithms" → translate
   - "Temporal types implementation details and testing" → translate

### Test Coverage Updates

Update test count from 17 to 25 tests throughout translations where mentioned.

### Files Not Requiring Translation

The following new files are English-only technical documentation:
- `docs/ORACLE_TYPE_MAPPING.md` - Technical reference (English only)
- `docs/TEMPORAL_TYPES.md` - Implementation details (English only)

These are intended for developers and do not require translation.

## Translation Priority

**High Priority Languages** (for manual review):
1. French (README.fr.md)
2. German (README.de.md)
3. Spanish (README.es.md)
4. Italian (README.it.md)
5. Chinese (README.zh.md)

**Lower Priority:**
- All other EU languages can be updated in batch

## Automation Notes

For automated translation updates:
1. Use the main `README.md` as the source of truth
2. Preserve all code blocks, commands, file paths unchanged
3. Only translate natural language descriptions
4. Maintain exact markdown structure
5. Update version numbers and dates as needed

## Change Log for Translators

| Date | Change | Impact |
|------|--------|--------|
| 2026-02-02 | Added INTERVAL types support | Add 2 new rows to type table |
| 2026-02-02 | Added JSON/XML type support | Add 2 new rows to type table |
| 2026-02-02 | Added technical doc references | Add 3 lines after type table |
| 2026-02-02 | Updated test count (17→25) | Update test mentions |
| 2026-02-01 | Added TIMESTAMP TZ support | Already in some translations |
| 2026-01-31 | Added RAW/Binary support | Already in some translations |
