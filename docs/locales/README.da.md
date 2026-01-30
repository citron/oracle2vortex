# oracle2vortex

En CLI-applikation, der ekstraherer Oracle-tabeller til Vortex-format via SQLcl med JSON-streaming.

## Beskrivelse

`oracle2vortex` muliggør eksport af Oracle-data ved hjælp af:
- **SQLcl** til forbindelse og native JSON-eksport
- **Streaming** til at behandle data i realtid uden at vente på eksportens afslutning
- **Automatisk konvertering** til søjlebaseret Vortex-format med skemainferens

✅ **Projekt afsluttet og testet i produktion** - Valideret med en tabel på 417 kolonner i en reel database.

## Forudsætninger

- **Rust nightly** (påkrævet af Vortex-crates)
- **SQLcl** installeret (eller angiv sti med `--sqlcl-path`)
- En tilgængelig Oracle-database

### Installation af Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Installation af SQLcl

Download SQLcl fra: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Eller på Linux:
```bash
# Eksempel på installation i /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Installation

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Den eksekverbare fil vil være tilgængelig i `target/release/oracle2vortex`.

## Brug

### Grundlæggende syntaks

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

### Indstillinger

| Indstilling | Kort | Beskrivelse | Standard |
|-------------|------|-------------|----------|
| `--sql-file` | `-f` | Sti til SQL-fil indeholdende forespørgslen | (påkrævet) |
| `--output` | `-o` | Sti til output Vortex-fil | (påkrævet) |
| `--host` | | Oracle-vært | (påkrævet) |
| `--port` | | Oracle-port | 1521 |
| `--user` | `-u` | Oracle-bruger | (påkrævet) |
| `--password` | `-p` | Oracle-adgangskode | (påkrævet) |
| `--sid` | | Oracle-SID eller servicenavn | (påkrævet) |
| `--sqlcl-path` | | Sti til SQLcl eksekverbar fil | `sql` |
| `--auto-batch-rows` | | Antal rækker pr. batch (0 = deaktiveret) | 0 |
| `--skip-lobs` | | Spring Oracle LOB-typer over (CLOB, BLOB, NCLOB) | false |

### Auto-Batching (Store tabeller)

For at behandle tabeller med millioner eller milliarder af rækker med konstant hukommelsesforbrug, brug indstillingen `--auto-batch-rows`:

```bash
# Behandl i batches på 50000 rækker
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

**Sådan virker det:**
1. Omslutter automatisk din forespørgsel med `OFFSET/FETCH`
2. Udfører SQLcl flere gange (én gang pr. batch)
3. Akkumulerer alle resultater i hukommelsen
4. Skriver en enkelt Vortex-fil indeholdende alle data

**Begrænsninger:**
- Kræver Oracle 12c+ (OFFSET/FETCH syntaks)
- Din forespørgsel må IKKE allerede indeholde OFFSET/FETCH eller ROWNUM
- Anbefalet: tilføj ORDER BY for konsekvent rækkefølge

**Hukommelse:** Med auto-batching, brugt hukommelse = batchstørrelse × 2 (JSON + Vortex)  
Eksempel: 50000 rækker × 1 KB = 100 MB pr. batch (i stedet for at indlæse hele tabellen)

**Se også:** `BATCH_PROCESSING.md` og `README_LARGE_DATASETS.md` for flere detaljer.

### Spring LOB-kolonner over

Oracle LOB-typer (CLOB, BLOB, NCLOB) kan være meget store og er måske ikke nødvendige til analyse. Brug `--skip-lobs` for at udelukke dem:

```bash
# Spring LOB-kolonner over for at reducere filstørrelsen og forbedre ydeevnen
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

**Sådan virker det:**
- Opdager og filtrerer automatisk kolonner indeholdende LOB-data
- LOB'er identificeres efter størrelse (> 4000 tegn) eller binære indikatorer
- Den første loggede post vil vise hvor mange kolonner der blev sprunget over
- Reducerer filstørrelsen og hukommelsesforbruget betydeligt for tabeller med store tekst-/binære felter

**Anvendelsestilfælde:**
- Eksport af metadatatabeller med beskrivelsesfelt
- Arbejde med tabeller indeholdende XML- eller store JSON-dokumenter
- Fokusere på strukturerede data mens binært indhold ignoreres
- Ydeevneoptimering for tabeller med mange store kolonner

### Eksempel med SQL-fil

Opret en fil `query.sql`:

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

Udfør derefter:

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

## Arkitektur

```
┌─────────────┐
│  SQL-fil    │
│             │
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
│  .vortex-fil             │
│  (columnar binary)       │
└──────────────────────────┘
```

## Sådan virker det

1. **SQL-læsning**: SQL-filen indlæses i hukommelsen
2. **SQLcl-start**: Processen starter med Oracle-forbindelse
3. **Sessionskonfiguration**:
   - `SET SQLFORMAT JSON` til JSON-eksport
   - `SET NLS_NUMERIC_CHARACTERS='.,';` for at undgå locale-problemer
4. **Forespørgselsudførelse**: SQL-forespørgslen sendes via stdin
5. **Output-opfangning**: Fuld læsning af JSON stdout
6. **JSON-ekstraktion**: Isolering af strukturen `{"results":[{"items":[...]}]}`
7. **Skemainferens**: Vortex-skemaet udledes automatisk fra den første post
8. **Postkonvertering**: Hvert JSON-objekt transformeres til Vortex-kolonner
9. **Filskrivning**: Binær Vortex-fil oprettes med Tokio-session

## Understøttede datatyper

Konvertering af JSON- til Vortex-typer er automatisk:

| JSON-type | Vortex-type | Nullable | Bemærkninger |
|-----------|-------------|----------|--------------|
| `null` | `Utf8` | ✅ | Udledt som nullable streng |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (heltal) | `Primitive(I64)` | ✅ | Detekteret med `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Detekteret med `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serialiseret som JSON-streng |
| `object` | `Utf8` | ✅ | Serialiseret som JSON-streng |

**Bemærk**: Alle typer er nullable for at håndtere Oracle NULL-værdier.

## Logning og fejlfinding

Applikationen bruger `tracing` til logs. Beskeder vises på stderr med logniveau.

Logs inkluderer:
- Oracle-forbindelse
- Antal behandlede poster
- Udledt skema
- Fejl og advarsler

## Verifikation af genererede Vortex-filer

For at verificere genererede filer, brug værktøjet `vx`:

```bash
# Installation af vx (Vortex CLI-værktøj)
cargo install vortex-vx

# Gennemse en Vortex-fil
vx browse output.vortex

# Vis metadata
vx info output.vortex
```

## Begrænsninger og overvejelser

- **Komplekse typer**: Indlejrede JSON-objekter og arrays serialiseres til strenge
- **In-memory buffer**: Poster bufres i øjeblikket før skrivning (fremtidig optimering mulig)
- **Fast skema**: Udledt kun fra den første post (efterfølgende poster skal matche)
- **Sikkerhed**: Adgangskode videregives som CLI-argument (synlig med `ps`). Brug miljøvariabler i produktion.
- **LOB-typer**: Som standard inkluderes LOB-kolonner (CLOB, BLOB, NCLOB). Brug `--skip-lobs` for at udelukke dem for bedre ydeevne og mindre filstørrelser.

## Udvikling

### Debug-build

```bash
cargo build
```

### Release-build

```bash
cargo build --release
```

Binærfilen vil være i `target/release/oracle2vortex` (~46 MB i release).

### Tests

```bash
cargo test
```

### Manuelle tests

Testfiler med legitimationsoplysninger er i `tests_local/` (gitignored):

```bash
# Opret testforespørgsler
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Udfør
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licens

Copyright (c) 2026 William Gacquer

Dette projekt er licenseret under EUPL-1.2 (European Union Public Licence v. 1.2).

**VIGTIGT - Begrænsning af kommerciel brug:**  
Kommerciel brug af denne software er forbudt uden forudgående skriftlig aftale med forfatteren.  
For enhver anmodning om kommerciel licens, kontakt: **oracle2vortex@amilto.com**

Se filen [LICENSE](LICENSE) for den fulde licenstekst.

## Forfatter

**William Gacquer**  
Kontakt: oracle2vortex@amilto.com

## Testhistorik

Projektet er blevet valideret på en Oracle-produktionsdatabase:

- ✅ **Simpel test**: 10 poster, 3 kolonner → 5,5 KB
- ✅ **Kompleks test**: 100 poster, 417 kolonner → 1,3 MB
- ✅ **Validering**: Filer læsbare med `vx browse` (Vortex v0.58)

## Projektstruktur

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Denne fil
├── IMPLEMENTATION.md       # Teknisk dokumentation
├── .gitignore             # Ekskluderer tests_local/ og legitimationsoplysninger
├── src/
│   ├── main.rs            # Entry point med tokio runtime
│   ├── cli.rs             # Clap argument parsing
│   ├── sqlcl.rs           # SQLcl-proces med CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # JSON→Vortex-konvertering (API 0.58)
│   └── pipeline.rs        # Fuldstændig orkestrering
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Eksempelforespørgsel
└── tests_local/           # Tests med legitimationsoplysninger (gitignored)
```

## Hovedafhængigheder

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Ressourcer

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
