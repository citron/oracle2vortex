# oracle2vortex

Een CLI-applicatie die Oracle-tabellen extraheert naar Vortex-formaat via SQLcl met JSON-streaming.

## Beschrijving

`oracle2vortex` maakt het exporteren van Oracle-gegevens mogelijk met:
- **SQLcl** voor verbinding en native JSON-export
- **Streaming** om gegevens direct te verwerken zonder te wachten op voltooiing van de export
- **Automatische conversie** naar kolomgebaseerd Vortex-formaat met schema-inferentie

✅ **Project voltooid en getest in productie** - Gevalideerd met een tabel van 417 kolommen op een echte database.

## Vereisten

- **Rust nightly** (vereist door Vortex crates)
- **SQLcl** geïnstalleerd (of specificeer pad met `--sqlcl-path`)
- Een toegankelijke Oracle-database

### Installatie van Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Installatie van SQLcl

Download SQLcl van: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Of op Linux:
```bash
# Voorbeeld voor installatie in /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Installatie

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Het uitvoerbare bestand is beschikbaar in `target/release/oracle2vortex`.

## Gebruik

### Basissyntaxis

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

### Opties

| Optie | Kort | Beschrijving | Standaard |
|-------|------|--------------|-----------|
| `--sql-file` | `-f` | Pad naar SQL-bestand met de query | (vereist) |
| `--output` | `-o` | Pad van uitvoer Vortex-bestand | (vereist) |
| `--host` | | Oracle-host | (vereist) |
| `--port` | | Oracle-poort | 1521 |
| `--user` | `-u` | Oracle-gebruiker | (vereist) |
| `--password` | `-p` | Oracle-wachtwoord | (vereist) |
| `--sid` | | Oracle-SID of servicenaam | (vereist) |
| `--sqlcl-path` | | Pad naar SQLcl-uitvoerbaar bestand | `sql` |
| `--auto-batch-rows` | | Aantal rijen per batch (0 = uitgeschakeld) | 0 |
| `--skip-lobs` | | Oracle LOB-typen overslaan (CLOB, BLOB, NCLOB) | false |

### Auto-Batching (Grote tabellen)

Om tabellen met miljoenen of miljarden rijen te verwerken met constant geheugengebruik, gebruik de optie `--auto-batch-rows`:

```bash
# Verwerk in batches van 50000 rijen
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

**Hoe het werkt:**
1. Omsluit uw query automatisch met `OFFSET/FETCH`
2. Voert SQLcl meerdere keren uit (eenmaal per batch)
3. Accumuleert alle resultaten in het geheugen
4. Schrijft één enkel Vortex-bestand met alle gegevens

**Beperkingen:**
- Vereist Oracle 12c+ (OFFSET/FETCH-syntaxis)
- Uw query mag NIET al OFFSET/FETCH of ROWNUM bevatten
- Aanbevolen: voeg ORDER BY toe voor consistente volgorde

**Geheugen:** Met auto-batching, gebruikt geheugen = batchgrootte × 2 (JSON + Vortex)  
Voorbeeld: 50000 rijen × 1 KB = 100 MB per batch (in plaats van de hele tabel laden)

**Zie ook:** `BATCH_PROCESSING.md` en `README_LARGE_DATASETS.md` voor meer details.

### LOB-kolommen overslaan

Oracle LOB-typen (CLOB, BLOB, NCLOB) kunnen zeer groot zijn en zijn mogelijk niet nodig voor analyse. Gebruik `--skip-lobs` om ze uit te sluiten:

```bash
# LOB-kolommen overslaan om bestandsgrootte te verminderen en prestaties te verbeteren
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

**Hoe het werkt:**
- Detecteert en filtert automatisch kolommen met LOB-gegevens
- LOB's worden geïdentificeerd op grootte (> 4000 tekens) of binaire indicatoren
- Het eerste geregistreerde record toont hoeveel kolommen zijn overgeslagen
- Vermindert de bestandsgrootte en het geheugengebruik aanzienlijk voor tabellen met grote tekst-/binaire velden

**Gebruiksscenario's:**
- Exporteren van metadatatabellen met beschrijvingsvelden
- Werken met tabellen die XML- of grote JSON-documenten bevatten
- Focussen op gestructureerde gegevens terwijl binaire inhoud wordt genegeerd
- Prestatieoptimalisatie voor tabellen met veel grote kolommen

### Voorbeeld met SQL-bestand

Maak een bestand `query.sql`:

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

Voer dan uit:

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

## Architectuur

```
┌─────────────┐
│  SQL-       │
│  bestand    │
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
│  .vortex-bestand         │
│  (columnar binary)       │
└──────────────────────────┘
```

## Werking

1. **SQL-lezen**: Het SQL-bestand wordt in het geheugen geladen
2. **SQLcl-start**: Proces start met Oracle-verbinding
3. **Sessieconfiguratie**:
   - `SET SQLFORMAT JSON` voor JSON-export
   - `SET NLS_NUMERIC_CHARACTERS='.,';` om locale-problemen te voorkomen
4. **Query-uitvoering**: De SQL-query wordt verzonden via stdin
5. **Uitvoer vastleggen**: Volledig lezen van JSON-stdout
6. **JSON-extractie**: Isolatie van de `{"results":[{"items":[...]}]}`-structuur
7. **Schema-inferentie**: Het Vortex-schema wordt automatisch afgeleid van het eerste record
8. **Record-conversie**: Elk JSON-object wordt getransformeerd naar Vortex-kolommen
9. **Bestand schrijven**: Binair Vortex-bestand gemaakt met Tokio-sessie

## Ondersteunde gegevenstypen

Conversie van JSON- naar Vortex-typen is automatisch:

| JSON-type | Vortex-type | Nullable | Opmerkingen |
|-----------|-------------|----------|-------------|
| `null` | `Utf8` | ✅ | Afgeleid als nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (geheel getal) | `Primitive(I64)` | ✅ | Gedetecteerd met `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Gedetecteerd met `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Geserialiseerd als JSON-string |
| `object` | `Utf8` | ✅ | Geserialiseerd als JSON-string |

**Opmerking**: Alle typen zijn nullable om Oracle NULL-waarden te verwerken.

## Logging en debugging

De applicatie gebruikt `tracing` voor logs. Berichten worden weergegeven op stderr met logniveau.

Logs bevatten:
- Oracle-verbinding
- Aantal verwerkte records
- Afgeleid schema
- Fouten en waarschuwingen

## Verificatie van gegenereerde Vortex-bestanden

Om gegenereerde bestanden te verifiëren, gebruik de `vx`-tool:

```bash
# Installatie van vx (Vortex CLI-tool)
cargo install vortex-vx

# Blader door een Vortex-bestand
vx browse output.vortex

# Toon metadata
vx info output.vortex
```

## Beperkingen en overwegingen

- **Complexe typen**: Geneste JSON-objecten en arrays worden geserialiseerd naar strings
- **In-memory buffer**: Records worden momenteel gebufferd voor schrijven (toekomstige optimalisatie mogelijk)
- **Vast schema**: Alleen afgeleid van het eerste record (volgende records moeten overeenkomen)
- **Beveiliging**: Wachtwoord wordt doorgegeven als CLI-argument (zichtbaar met `ps`). Gebruik omgevingsvariabelen in productie.
- **LOB-typen**: Standaard worden LOB-kolommen (CLOB, BLOB, NCLOB) opgenomen. Gebruik `--skip-lobs` om ze uit te sluiten voor betere prestaties en kleinere bestandsgroottes.

## Ontwikkeling

### Debug-build

```bash
cargo build
```

### Release-build

```bash
cargo build --release
```

Het binaire bestand bevindt zich in `target/release/oracle2vortex` (~46 MB in release).

### Tests

```bash
cargo test
```

### Handmatige tests

Testbestanden met inloggegevens bevinden zich in `tests_local/` (gitignored):

```bash
# Maak testqueries
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Uitvoeren
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licentie

Copyright (c) 2026 William Gacquer

Dit project is gelicentieerd onder EUPL-1.2 (European Union Public Licence v. 1.2).

**BELANGRIJK - Beperking commercieel gebruik:**  
Commercieel gebruik van deze software is verboden zonder voorafgaande schriftelijke toestemming van de auteur.  
Voor elk commercieel licentieverzoek, neem contact op met: **oracle2vortex@amilto.com**

Zie het [LICENSE](LICENSE)-bestand voor de volledige licentietekst.

## Auteur

**William Gacquer**  
Contact: oracle2vortex@amilto.com

## Testgeschiedenis

Het project is gevalideerd op een Oracle-productiedatabase:

- ✅ **Eenvoudige test**: 10 records, 3 kolommen → 5,5 KB
- ✅ **Complexe test**: 100 records, 417 kolommen → 1,3 MB
- ✅ **Validatie**: Bestanden leesbaar met `vx browse` (Vortex v0.58)

## Projectstructuur

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Dit bestand
├── IMPLEMENTATION.md       # Technische documentatie
├── .gitignore             # Sluit tests_local/ en inloggegevens uit
├── src/
│   ├── main.rs            # Entry point met tokio runtime
│   ├── cli.rs             # Clap argument parsing
│   ├── sqlcl.rs           # SQLcl-proces met CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # JSON→Vortex-conversie (API 0.58)
│   └── pipeline.rs        # Volledige orkestratie
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Voorbeeldquery
└── tests_local/           # Tests met inloggegevens (gitignored)
```

## Hoofdafhankelijkheden

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Bronnen

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
