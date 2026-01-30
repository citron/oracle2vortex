# oracle2vortex

En CLI-applikation som extraherar Oracle-tabeller till Vortex-format via SQLcl med JSON-strömning.

## Beskrivning

`oracle2vortex` möjliggör export av Oracle-data med:
- **SQLcl** för anslutning och inbyggd JSON-export
- **Strömning** för att bearbeta data i realtid utan att vänta på exportens slutförande
- **Automatisk konvertering** till kolumnbaserat Vortex-format med schemainferens

✅ **Projekt avslutat och testat i produktion** - Validerat med en tabell på 417 kolumner i en riktig databas.

## Förutsättningar

- **Rust nightly** (krävs av Vortex-crates)
- **SQLcl** installerad (eller ange sökväg med `--sqlcl-path`)
- En tillgänglig Oracle-databas

### Installation av Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Installation av SQLcl

Ladda ner SQLcl från: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Eller på Linux:
```bash
# Exempel för installation i /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Installation

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Den körbara filen kommer att finnas i `target/release/oracle2vortex`.

## Användning

### Grundläggande syntax

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

### Alternativ

| Alternativ | Kort | Beskrivning | Standard |
|------------|------|-------------|----------|
| `--sql-file` | `-f` | Sökväg till SQL-fil som innehåller frågan | (krävs) |
| `--output` | `-o` | Sökväg för utdata-Vortex-fil | (krävs) |
| `--host` | | Oracle-värd | (krävs) |
| `--port` | | Oracle-port | 1521 |
| `--user` | `-u` | Oracle-användare | (krävs) |
| `--password` | `-p` | Oracle-lösenord | (krävs) |
| `--sid` | | Oracle-SID eller tjänstnamn | (krävs) |
| `--sqlcl-path` | | Sökväg till SQLcl körbar fil | `sql` |
| `--auto-batch-rows` | | Antal rader per batch (0 = inaktiverad) | 0 |

### Auto-Batching (Stora tabeller)

För att bearbeta tabeller med miljoner eller miljarder rader med konstant minnesanvändning, använd alternativet `--auto-batch-rows`:

```bash
# Bearbeta i batcher på 50000 rader
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

**Hur det fungerar:**
1. Omsluter automatiskt din fråga med `OFFSET/FETCH`
2. Kör SQLcl flera gånger (en gång per batch)
3. Ackumulerar alla resultat i minnet
4. Skriver en enda Vortex-fil som innehåller all data

**Begränsningar:**
- Kräver Oracle 12c+ (OFFSET/FETCH-syntax)
- Din fråga får INTE redan innehålla OFFSET/FETCH eller ROWNUM
- Rekommenderat: lägg till ORDER BY för konsekvent ordning

**Minne:** Med auto-batching, använt minne = batchstorlek × 2 (JSON + Vortex)  
Exempel: 50000 rader × 1 KB = 100 MB per batch (istället för att ladda hela tabellen)

**Se även:** `BATCH_PROCESSING.md` och `README_LARGE_DATASETS.md` för mer detaljer.

### Exempel med SQL-fil

Skapa en fil `query.sql`:

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

Kör sedan:

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

## Hur det fungerar

1. **SQL-läsning**: SQL-filen laddas in i minnet
2. **SQLcl-start**: Processen startar med Oracle-anslutning
3. **Sessionskonfiguration**:
   - `SET SQLFORMAT JSON` för JSON-export
   - `SET NLS_NUMERIC_CHARACTERS='.,';` för att undvika lokaliseringsproblem
4. **Frågekörning**: SQL-frågan skickas via stdin
5. **Utdatafångst**: Fullständig läsning av JSON stdout
6. **JSON-extraktion**: Isolering av strukturen `{"results":[{"items":[...]}]}`
7. **Schemainferens**: Vortex-schemat härleds automatiskt från den första posten
8. **Postkonvertering**: Varje JSON-objekt transformeras till Vortex-kolumner
9. **Filskrivning**: Binär Vortex-fil skapas med Tokio-session

## Stödda datatyper

Konvertering av JSON- till Vortex-typer är automatisk:

| JSON-typ | Vortex-typ | Nullable | Anteckningar |
|----------|------------|----------|--------------|
| `null` | `Utf8` | ✅ | Härledas som nullable sträng |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (heltal) | `Primitive(I64)` | ✅ | Detekterad med `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Detekterad med `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serialiserad som JSON-sträng |
| `object` | `Utf8` | ✅ | Serialiserad som JSON-sträng |

**Obs**: Alla typer är nullable för att hantera Oracle NULL-värden.

## Loggning och felsökning

Applikationen använder `tracing` för loggar. Meddelanden visas på stderr med loggnivå.

Loggar inkluderar:
- Oracle-anslutning
- Antal bearbetade poster
- Härlett schema
- Fel och varningar

## Verifiering av genererade Vortex-filer

För att verifiera genererade filer, använd verktyget `vx`:

```bash
# Installation av vx (Vortex CLI-verktyg)
cargo install vortex-vx

# Bläddra i en Vortex-fil
vx browse output.vortex

# Visa metadata
vx info output.vortex
```

## Begränsningar och överväganden

- **Komplexa typer**: Nästlade JSON-objekt och arrayer serialiseras till strängar
- **Minnesbuffert**: Poster buffras för närvarande före skrivning (framtida optimering möjlig)
- **Fast schema**: Härlett endast från första posten (efterföljande poster måste matcha)
- **Säkerhet**: Lösenord skickas som CLI-argument (synligt med `ps`). Använd miljövariabler i produktion.

## Utveckling

### Debug-build

```bash
cargo build
```

### Release-build

```bash
cargo build --release
```

Binärfilen kommer att finnas i `target/release/oracle2vortex` (~46 MB i release).

### Tester

```bash
cargo test
```

### Manuella tester

Testfiler med autentiseringsuppgifter finns i `tests_local/` (gitignored):

```bash
# Skapa testfrågor
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Kör
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

Detta projekt är licensierat under EUPL-1.2 (European Union Public Licence v. 1.2).

**VIKTIGT - Begränsning av kommersiell användning:**  
Kommersiell användning av denna programvara är förbjuden utan föregående skriftligt avtal med författaren.  
För alla förfrågningar om kommersiell licens, kontakta: **oracle2vortex@amilto.com**

Se filen [LICENSE](LICENSE) för fullständig licenstext.

## Författare

**William Gacquer**  
Kontakt: oracle2vortex@amilto.com

## Testhistorik

Projektet har validerats på en Oracle-produktionsdatabas:

- ✅ **Enkelt test**: 10 poster, 3 kolumner → 5,5 KB
- ✅ **Komplext test**: 100 poster, 417 kolumner → 1,3 MB
- ✅ **Validering**: Filer läsbara med `vx browse` (Vortex v0.58)

## Projektstruktur

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Denna fil
├── IMPLEMENTATION.md       # Teknisk dokumentation
├── .gitignore             # Exkluderar tests_local/ och autentiseringsuppgifter
├── src/
│   ├── main.rs            # Entry point med tokio runtime
│   ├── cli.rs             # Clap argument parsing
│   ├── sqlcl.rs           # SQLcl-process med CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # JSON→Vortex-konvertering (API 0.58)
│   └── pipeline.rs        # Fullständig orkestrering
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Exempelfråga
└── tests_local/           # Tester med autentiseringsuppgifter (gitignored)
```

## Huvudberoenden

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Resurser

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
