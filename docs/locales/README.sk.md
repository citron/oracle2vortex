# oracle2vortex

CLI aplikácia na export Oracle tabuliek do formátu Vortex cez SQLcl so streamovaním JSON.

## Popis

`oracle2vortex` umožňuje export dát z Oracle pomocou:
- **SQLcl** pre pripojenie a natívny JSON export
- **Streamovanie** pre spracovanie dát za behu bez čakania na ukončenie exportu
- **Automatická konverzia** do stĺpcového formátu Vortex s inferenciou schémy

✅ **Projekt dokončený a testovaný v produkcii** - Validovaný s tabuľkou 417 stĺpcov na reálnej databáze.

## Predpoklady

- **Rust nightly** (vyžadované Vortex crate-mi)
- **SQLcl** nainštalované (alebo zadajte cestu pomocou `--sqlcl-path`)
- Prístupná Oracle databáza

### Inštalácia Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Inštalácia SQLcl

Stiahnite SQLcl z: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Alebo na Linuxe:
```bash
# Príklad inštalácie do /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Inštalácia

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Spustiteľný súbor bude dostupný v `target/release/oracle2vortex`.

## Použitie

### Základná syntax

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

### Možnosti

| Možnosť | Krátka | Popis | Predvolené |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Cesta k SQL súboru obsahujúcemu dotaz | (povinné) |
| `--output` | `-o` | Cesta výstupného Vortex súboru | (povinné) |
| `--host` | | Oracle hostiteľ | (povinné) |
| `--port` | | Oracle port | 1521 |
| `--user` | `-u` | Oracle používateľ | (povinné) |
| `--password` | `-p` | Oracle heslo | (povinné) |
| `--sid` | | Oracle SID alebo názov služby | (povinné) |
| `--sqlcl-path` | | Cesta k spustiteľnému SQLcl | `sql` |
| `--auto-batch-rows` | | Počet riadkov na dávku (0 = vypnuté) | 0 |

### Auto-dávkovanie (veľké tabuľky)

Pre zpracovanie tabuliek s miliónmi alebo miliardami riadkov s konštantným využitím pamäte použite možnosť `--auto-batch-rows`:

```bash
# Spracovanie v dávkach po 50000 riadkoch
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

**Ako to funguje:**
1. Automaticky obalí dotaz pomocou `OFFSET/FETCH`
2. Spustí SQLcl viackrát (raz na dávku)
3. Nahromadí všetky výsledky v pamäti
4. Zapíše jeden Vortex súbor obsahujúci všetky dáta

**Obmedzenia:**
- Vyžaduje Oracle 12c+ (syntax OFFSET/FETCH)
- Váš dotaz nesmie už obsahovať OFFSET/FETCH alebo ROWNUM
- Odporúčané: pridajte ORDER BY pre konzistentné poradie

**Pamäť:** S auto-dávkovaním, použitá pamäť = veľkosť dávky × 2 (JSON + Vortex)  
Príklad: 50000 riadkov × 1 KB = 100 MB na dávku (namiesto načítania celej tabuľky)

**Pozrite tiež:** `BATCH_PROCESSING.md` a `README_LARGE_DATASETS.md` pre ďalšie detaily.

### Preskočenie LOB stĺpcov

Oracle LOB typy (CLOB, BLOB, NCLOB) môžu byť veľmi veľké a nemusia byť potrebné na analýzu. Použite `--skip-lobs` na ich vylúčenie:

```bash
# Preskočte LOB stĺpce na zníženie veľkosti súboru a zlepšenie výkonu
oracle2vortex \
  -f query.sql \
  -o data.vortex \
  --host db.example.com \
  --port 1521 \
  -u hr \
  -p secret123 \
  --sid PROD \
  --skip-lobs
```

**Ako to funguje:**
- Automaticky deteguje a filtruje stĺpce obsahujúce LOB dáta
- LOB sa identifikujú podľa veľkosti (> 4000 znakov) alebo binárnych indikátorov
- Prvý zaznamenaný záznam zobrazí, koľko stĺpcov bolo preskočených
- Výrazne znižuje veľkosť súboru a využitie pamäte pre tabuľky s veľkými textovými/binárnymi poľami

**Prípady použitia:**
- Export tabuliek metadát s poľami popisu
- Práca s tabuľkami obsahujúcimi XML alebo veľké JSON dokumenty
- Zameranie na štruktúrované dáta pri ignorovaní binárneho obsahu
- Optimalizácia výkonu pre tabuľky s mnohými veľkými stĺpcami

### Príklad so SQL súborom

Vytvorte súbor `query.sql`:

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

Potom spustite:

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

## Architektúra

```
┌─────────────┐
│  SQL        │
│  súbor      │
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
│  .vortex súbor           │
│  (columnar binary)       │
└──────────────────────────┘
```

## Fungovanie

1. **Čítanie SQL**: SQL súbor je načítaný do pamäte
2. **Spustenie SQLcl**: Spustenie procesu s Oracle pripojením
3. **Konfigurácia relácie**:
   - `SET SQLFORMAT JSON` pre JSON export
   - `SET NLS_NUMERIC_CHARACTERS='.,';` pre zabránenie problémom s locale
4. **Vykonanie dotazu**: SQL dotaz je odoslaný cez stdin
5. **Zachytenie výstupu**: Kompletné čítanie JSON stdout
6. **Extrakcia JSON**: Izolácia štruktúry `{"results":[{"items":[...]}]}`
7. **Inferencia schémy**: Vortex schéma je automaticky odvodená z prvého záznamu
8. **Konverzia záznamov**: Každý JSON objekt je transformovaný na Vortex stĺpce
9. **Zápis súboru**: Binárny Vortex súbor je vytvorený s Tokio session

## Podporované dátové typy

Konverzia JSON typov na Vortex typy prebieha automaticky:

| JSON typ | Vortex typ | Nullable | Poznámky |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Odvodený ako nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (celé číslo) | `Primitive(I64)` | ✅ | Detekované pomocou `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Detekované pomocou `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serializované ako JSON reťazec |
| `object` | `Utf8` | ✅ | Serializované ako JSON reťazec |

**Poznámka**: Všetky typy sú nullable pre spracovanie Oracle NULL hodnôt.

## Logy a ladenie

Aplikácia používa `tracing` pre logy. Správy sú zobrazené na stderr s úrovňou logu.

Logy zahŕňajú:
- Pripojenie k Oracle
- Počet spracovaných záznamov
- Odvodená schéma
- Chyby a upozornenia

## Overenie generovaných Vortex súborov

Pre overenie generovaných súborov použite nástroj `vx`:

```bash
# Inštalácia vx (Vortex CLI nástroj)
cargo install vortex-vx

# Prehliadanie Vortex súboru
vx browse output.vortex

# Zobrazenie metadát
vx info output.vortex
```

## Obmedzenia a úvahy

- **Zložité typy**: Vnorené JSON objekty a polia sú serializované do reťazcov
- **Buffer v pamäti**: Záznamy sú aktuálne bufferované pred zápisom (budúca optimalizácia možná)
- **Pevná schéma**: Odvodená len z prvého záznamu (nasledujúce záznamy musia zodpovedať)
- **Bezpečnosť**: Heslo je predané ako CLI argument (viditeľné pomocou `ps`). V produkcii používajte premenné prostredia.
- **LOB typy**: Predvolene sú LOB stĺpce (CLOB, BLOB, NCLOB) zahrnuté. Použite `--skip-lobs` na ich vylúčenie pre lepší výkon a menšie veľkosti súborov.

## Vývoj

### Build v debug režime

```bash
cargo build
```

### Build v release režime

```bash
cargo build --release
```

Binárka bude v `target/release/oracle2vortex` (~46 MB v release).

### Testy

```bash
cargo test
```

### Manuálne testy

Testovacie súbory s prihlasovacími údajmi sú v `tests_local/` (gitignored):

```bash
# Vytvorenie testovacích dotazov
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Spustenie
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licencia

Copyright (c) 2026 William Gacquer

Tento projekt je licencovaný pod EUPL-1.2 (European Union Public Licence v. 1.2).

**DÔLEŽITÉ - Obmedzenie komerčného použitia:**  
Komerčné použitie tohto softvéru je zakázané bez predchádzajúceho písomného súhlasu autora.  
Pre žiadosti o komerčnú licenciu kontaktujte: **oracle2vortex@amilto.com**

Pozri súbor [LICENSE](LICENSE) pre úplný text licencie.

## Autor

**William Gacquer**  
Kontakt: oracle2vortex@amilto.com

## História testov

Projekt bol validovaný na produkčnej Oracle databáze:

- ✅ **Jednoduchý test**: 10 záznamov, 3 stĺpce → 5.5 KB
- ✅ **Zložitý test**: 100 záznamov, 417 stĺpcov → 1.3 MB
- ✅ **Validácia**: Súbory čitateľné pomocou `vx browse` (Vortex v0.58)

## Štruktúra projektu

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Tento súbor
├── IMPLEMENTATION.md       # Technická dokumentácia
├── .gitignore             # Vylučuje tests_local/ a prihlasovacie údaje
├── src/
│   ├── main.rs            # Entry point s tokio runtime
│   ├── cli.rs             # Spracovanie argumentov Clap
│   ├── sqlcl.rs           # SQLcl proces s CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Konverzia JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Kompletná orchestrácia
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Ukážkový dotaz
└── tests_local/           # Testy s prihlasovacími údajmi (gitignored)
```

## Hlavné závislosti

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
