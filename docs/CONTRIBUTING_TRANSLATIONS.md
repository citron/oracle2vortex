# Contributing Translations

Thank you for your interest in translating oracle2vortex documentation!

## Current Translation Status

- ✅ **English** (README.md) - Always current (authoritative)
- ✅ **French** (docs/locales/README.fr.md) - Updated 2026-02-02
- ⚠️ **24 other languages** - Need updates for recent features

See [`TRANSLATION_UPDATE_NOTES.md`](TRANSLATION_UPDATE_NOTES.md) for a detailed list of changes.

## Quick Start for Translators

### 1. Choose a Language

Check the current status in [`TRANSLATIONS.md`](TRANSLATIONS.md).

Priority languages for updates:
- German (README.de.md)
- Spanish (README.es.md)
- Italian (README.it.md)
- Chinese (README.zh.md)
- Portuguese (README.pt.md)

### 2. Use French as a Reference

The French translation (`README.fr.md`) is fully up-to-date and can serve as a reference for the new content structure.

### 3. What to Translate

**Always translate:**
- Headings and section titles
- Descriptive text and explanations
- Table labels and notes
- Example descriptions

**Never translate:**
- Code blocks and commands
- File paths and URLs
- Email addresses
- Data type names (e.g., `INTERVAL DAY TO SECOND`, `JSON`, `BLOB`)
- SQL format strings (e.g., `"YYYY-MM-DD"`)
- Technical constants (e.g., `I64`, `F64`, `VarBinArray`)

### 4. Maintain Structure

- Keep exact same markdown structure
- Preserve code fences (```)
- Keep table formatting identical
- Don't change link targets

## Key Sections Needing Updates

### Type Mapping Table

The main change is the expanded type mapping table in the "Supported data types" section. It now includes:

**New types:**
- `INTERVAL DAY TO SECOND`
- `INTERVAL YEAR TO MONTH`
- `JSON` (Oracle 21c+)
- `XMLTYPE`
- Expanded temporal types section

**New subsections:**
- "Temporal Types with Timezone Support"
- "Binary Data (RAW/BLOB)"

### Documentation References

New paragraph after type table:

```markdown
**For detailed type mapping algorithms and detection logic, see:**
- [`docs/ORACLE_TYPE_MAPPING.md`](../ORACLE_TYPE_MAPPING.md) - Complete technical reference
- [`docs/TEMPORAL_TYPES.md`](../TEMPORAL_TYPES.md) - Temporal types implementation
```

(Translate the descriptions, keep file paths)

### Test Count

Update "17 tests" to "25 tests" where mentioned.

## Example: Type Table Row Translation

**English (source):**
```markdown
| `INTERVAL DAY TO SECOND` | `"+02 02:30:00.123456"` | `Primitive(I64)` | I64 | Total microseconds |
```

**French (correct):**
```markdown
| `INTERVAL DAY TO SECOND` | `"+02 02:30:00.123456"` | `Primitive(I64)` | I64 | Microsecondes totales |
```

**What changed:**
- ✅ Type name: `INTERVAL DAY TO SECOND` - unchanged
- ✅ Example value: `"+02 02:30:00.123456"` - unchanged  
- ✅ Vortex type: `Primitive(I64)` - unchanged
- ✅ Storage: `I64` - unchanged
- ✅ Description: "Total microseconds" → "Microsecondes totales" - **translated**

## Translation Tools

### Recommended Approach

1. **Compare**: Use a diff tool to see changes between old README.fr.md and main README.md
2. **Copy**: Start with the existing translation
3. **Update**: Add new sections/rows from the updated French version
4. **Verify**: Check all code blocks are unchanged
5. **Test**: Ensure markdown renders correctly

### Automated Translation

If using automated tools (DeepL, Google Translate, etc.):
1. Translate ONLY natural language sections
2. Manually verify technical terms are unchanged
3. Check markdown formatting is preserved
4. Review the French version for reference

## Submission

1. Fork the repository
2. Update your language file in `docs/locales/`
3. Verify markdown renders correctly
4. Submit a pull request with:
   - Title: `docs: Update [Language] translation`
   - Description: List of sections updated

## Questions?

- Check [`TRANSLATION_UPDATE_NOTES.md`](TRANSLATION_UPDATE_NOTES.md) for detailed change log
- Look at `README.fr.md` for a complete up-to-date example
- Open an issue if you need clarification

Thank you for helping make oracle2vortex accessible worldwide! 🌍
