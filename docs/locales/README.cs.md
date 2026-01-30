# oracle2vortex

CLI aplikace pro export Oracle tabulek do formátu Vortex pomocí SQLcl se streamováním JSON.

## Popis

`oracle2vortex` umožňuje export dat z Oracle pomocí:
- **SQLcl** pro připojení a nativní export do JSON
- **Streamování** pro zpracování dat za běhu bez čekání na ukončení exportu
- **Automatická konverze** do sloupcového formátu Vortex s odvozením schématu

✅ **Projekt dokončen a testován v produkci** - Ověřeno na tabulce se 417 sloupci v reálné databázi.

## Předpoklady

- **Rust nightly** (vyžadováno Vortex crates)
- **SQLcl** nainstalováno (nebo zadejte cestu pomocí `--sqlcl-path`)
- Přístupná databáze Oracle

### Instalace Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Instalace SQLcl

Stáhněte SQLcl z: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Nebo na Linuxu:
```bash
# Příklad instalace do /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Instalace

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Spustitelný soubor bude k dispozici v `target/release/oracle2vortex`.

## Použití

### Základní syntaxe

```bash
oracle2vortex \
  --sql-file query.sql \
  --output data.vortex \
  --host localhost \
  --port 1521 \
  --user hr \
  --password mypassword \
  --sid ORCL
```

### Volby

| Volba | Krátká | Popis | Výchozí |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Cesta k SQL souboru obsahujícímu dotaz | (povinné) |
| `--output` | `-o` | Cesta k výstupnímu Vortex souboru | (povinné) |
| `--host` | | Oracle hostitel | (povinné) |
| `--port` | | Oracle port | 1521 |
| `--user` | `-u` | Oracle uživatel | (povinné) |
| `--password` | `-p` | Oracle heslo | (povinné) |
| `--sid` | | Oracle SID nebo název služby | (povinné) |
| `--sqlcl-path` | | Cesta ke spustitelnému SQLcl | `sql` |
| `--auto-batch-rows` | | Počet řádků na dávku (0 = vypnuto) | 0 |

### Auto-dávkování (velké tabulky)

Pro zpracování tabulek s miliony nebo miliardami řádků s konstantním využitím paměti použijte volbu `--auto-batch-rows`:

```bash
# Zpracování v dávkách po 50000 řádcích
oracle2vortex \
  -f query.sql \
  -o data.vortex \
  --host db.example.com \
  --port 1521 \
  -u hr \
  -p secret123 \
  --sid PROD \
  --auto-batch-rows 50000
```

**Jak to funguje:**
1. Automaticky obalí váš dotaz pomocí `OFFSET/FETCH`
2. Spustí SQLcl vícekrát (jednou na dávku)
3. Nahromadí všechny výsledky v paměti
4. Zapíše jeden Vortex soubor obsahující všechna data

**Omezení:**
- Vyžaduje Oracle 12c+ (syntaxe OFFSET/FETCH)
- Váš dotaz nesmí již obsahovat OFFSET/FETCH nebo ROWNUM
- Doporučeno: přidat ORDER BY pro konzistentní pořadí

**Paměť:** S auto-dávkováním, použitá paměť = velikost dávky × 2 (JSON + Vortex)  
Příklad: 50000 řádků × 1 KB = 100 MB na dávku (místo načtení celé tabulky)

**Viz také:** `BATCH_PROCESSING.md` a `README_LARGE_DATASETS.md` pro další detaily.

### Příklad s SQL souborem

Vytvořte soubor `query.sql`:

```sql
SELECT 
    employee_id,
    first_name,
    last_name,
    salary,
    hire_date
FROM employees
WHERE department_id = 50;
```

Poté spusťte:

```bash
oracle2vortex \
  -f query.sql \
  -o employees.vortex \
  --host db.example.com \
  --port 1521 \
  -u hr \
  -p secret123 \
  --sid PROD
```

## Architektura

```
┌─────────────┐
│  SQL        │
│  soubor     │
└──────┬──────┘
       │
       v
┌──────────────────────────┐
│  oracle2vortex CLI       │
│  (Clap argument parser)  │
└──────────┬───────────────┘
           │
           v
┌──────────────────────────┐
│  SQLcl Process           │
│  (CONNECT, SET FORMAT)   │
└──────────┬───────────────┘
           │ JSON: {"results":[{"items":[...]}]}
           v
┌──────────────────────────┐
│  JSON Stream Parser      │
│  (extraction + parsing)  │
└──────────┬───────────────┘
           │ Vec<serde_json::Value>
           v
┌──────────────────────────┐
│  Vortex Writer           │
│  (schema inference +     │
│   ArrayData construction)│
└──────────┬───────────────┘
           │ Vortex format
           v
┌──────────────────────────┐
│  .vortex soubor          │
│  (columnar binary)       │
└──────────────────────────┘
```

## Fungování

1. **Čtení SQL**: SQL soubor je načten do paměti
2. **Spuštění SQLcl**: Spuštění procesu s Oracle připojením
3. **Konfigurace relace**:
   - `SET SQLFORMAT JSON` pro export JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` pro zabránění problémům s locale
4. **Provedení dotazu**: SQL dotaz je odeslán přes stdin
5. **Zachycení výstupu**: Kompletní čtení JSON stdout
6. **Extrakce JSON**: Izolace struktury `{"results":[{"items":[...]}]}`
7. **Odvození schématu**: Vortex schéma je automaticky odvozeno z prvního záznamu
8. **Konverze záznamů**: Každý JSON objekt je transformován na Vortex sloupce
9. **Zápis souboru**: Binární Vortex soubor je vytvořen s Tokio session

## Podporované datové typy

Konverze JSON typů na Vortex typy probíhá automaticky:

| JSON typ | Vortex typ | Nullable | Poznámky |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Odvozeno jako nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (celé číslo) | `Primitive(I64)` | ✅ | Detekováno pomocí `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Detekováno pomocí `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serializováno jako JSON řetězec |
| `object` | `Utf8` | ✅ | Serializováno jako JSON řetězec |

**Poznámka**: Všechny typy jsou nullable pro zpracování Oracle NULL hodnot.

## Logy a ladění

Aplikace používá `tracing` pro logy. Zprávy jsou zobrazeny na stderr s úrovní logu.

Logy zahrnují:
- Připojení k Oracle
- Počet zpracovaných záznamů
- Odvozené schéma
- Chyby a varování

## Ověření generovaných Vortex souborů

Pro ověření generovaných souborů použijte nástroj `vx`:

```bash
# Instalace vx (Vortex CLI nástroj)
cargo install vortex-vx

# Procházení Vortex souboru
vx browse output.vortex

# Zobrazení metadat
vx info output.vortex
```

## Omezení a úvahy

- **Složité typy**: Vnořené JSON objekty a pole jsou serializovány do řetězců
- **Buffer v paměti**: Záznamy jsou aktuálně bufferovány před zápisem (budoucí optimalizace možná)
- **Pevné schéma**: Odvozeno pouze z prvního záznamu (následující záznamy musí odpovídat)
- **Bezpečnost**: Heslo je předáno jako CLI argument (viditelné pomocí `ps`). V produkci používejte proměnné prostředí.

## Vývoj

### Build v debug režimu

```bash
cargo build
```

### Build v release režimu

```bash
cargo build --release
```

Binárka bude v `target/release/oracle2vortex` (~46 MB v release).

### Testy

```bash
cargo test
```

### Manuální testy

Testovací soubory s přihlašovacími údaji jsou v `tests_local/` (gitignored):

```bash
# Vytvoření testovacích dotazů
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Spuštění
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licence

Copyright (c) 2026 William Gacquer

Tento projekt je licencován pod EUPL-1.2 (European Union Public Licence v. 1.2).

**DŮLEŽITÉ - Omezení komerčního použití:**  
Komerční použití tohoto softwaru je zakázáno bez předchozího písemného souhlasu autora.  
Pro žádosti o komerční licenci kontaktujte: **oracle2vortex@amilto.com**

Viz soubor [LICENSE](LICENSE) pro úplný text licence.

## Autor

**William Gacquer**  
Kontakt: oracle2vortex@amilto.com

## Historie testů

Projekt byl ověřen na produkční Oracle databázi:

- ✅ **Jednoduchý test**: 10 záznamů, 3 sloupce → 5.5 KB
- ✅ **Složitý test**: 100 záznamů, 417 sloupců → 1.3 MB
- ✅ **Validace**: Soubory čitelné pomocí `vx browse` (Vortex v0.58)

## Struktura projektu

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Tento soubor
├── IMPLEMENTATION.md       # Technická dokumentace
├── .gitignore             # Vylučuje tests_local/ a přihlašovací údaje
├── src/
│   ├── main.rs            # Entry point s tokio runtime
│   ├── cli.rs             # Zpracování argumentů Clap
│   ├── sqlcl.rs           # SQLcl proces s CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Konverze JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Kompletní orchestrace
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Ukázkový dotaz
└── tests_local/           # Testy s přihlašovacími údaji (gitignored)
```

## Hlavní závislosti

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Zdroje

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
