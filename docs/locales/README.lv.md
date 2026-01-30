# oracle2vortex

CLI lietotne Oracle tabulu eksportēšanai Vortex formātā caur SQLcl ar JSON straumēšanu.

## Apraksts

`oracle2vortex` ļauj eksportēt Oracle datus izmantojot:
- **SQLcl** savienojumam un iedzimtam JSON eksportam
- **Straumēšanu** datu apstrādei lidojumā, negaidot eksporta pabeigšanu
- **Automātisko konvertāciju** kolonu Vortex formātā ar shēmas secinājumu

✅ **Projekts pabeigts un pārbaudīts ražošanā** - Validēts ar 417 kolonnu tabulu uz reālas datubāzes.

## Priekšnosacījumi

- **Rust nightly** (nepieciešams Vortex crate-iem)
- **SQLcl** instalēts (vai norādiet ceļu ar `--sqlcl-path`)
- Pieejama Oracle datubāze

### Rust nightly instalēšana

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### SQLcl instalēšana

Lejupielādējiet SQLcl no: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Vai Linux sistēmā:
```bash
# Piemērs instalēšanai /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Instalēšana

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Izpildāmais fails būs pieejams `target/release/oracle2vortex`.

## Lietošana

### Pamata sintakse

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

### Opcijas

| Opcija | Īsā | Apraksts | Noklusējums |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Ceļš uz SQL failu ar vaicājumu | (obligāts) |
| `--output` | `-o` | Izvades Vortex faila ceļš | (obligāts) |
| `--host` | | Oracle resursdators | (obligāts) |
| `--port` | | Oracle ports | 1521 |
| `--user` | `-u` | Oracle lietotājs | (obligāts) |
| `--password` | `-p` | Oracle parole | (obligāts) |
| `--sid` | | Oracle SID vai servisa nosaukums | (obligāts) |
| `--sqlcl-path` | | Ceļš uz SQLcl izpildāmo failu | `sql` |
| `--auto-batch-rows` | | Rindu skaits partijā (0 = atspējots) | 0 |

### Automātiskā partiju apstrāde (lielas tabulas)

Miljoniem vai miljardiem rindu tabulu apstrādei ar konstantu atmiņas izmantošanu izmantojiet opciju `--auto-batch-rows`:

```bash
# Apstrāde 50000 rindu partijās
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

**Kā tas darbojas:**
1. Automātiski ietver jūsu vaicājumu ar `OFFSET/FETCH`
2. Izpilda SQLcl vairākas reizes (vienu reizi partijā)
3. Uzkrāj visus rezultātus atmiņā
4. Raksta vienu Vortex failu, kas satur visus datus

**Ierobežojumi:**
- Nepieciešams Oracle 12c+ (OFFSET/FETCH sintakse)
- Jūsu vaicājums NEDRĪKST jau saturēt OFFSET/FETCH vai ROWNUM
- Ieteicams: pievienojiet ORDER BY konsekventai secībai

**Atmiņa:** Ar automātisko partiju apstrādi, izmantotā atmiņa = partijas izmērs × 2 (JSON + Vortex)  
Piemērs: 50000 rindas × 1 KB = 100 MB partijā (nevis ielādējot visu tabulu)

**Skatiet arī:** `BATCH_PROCESSING.md` un `README_LARGE_DATASETS.md` papildu detaļām.

### Piemērs ar SQL failu

Izveidojiet failu `query.sql`:

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

Pēc tam izpildiet:

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

## Arhitektūra

```
┌─────────────┐
│  SQL        │
│  fails      │
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
│  .vortex fails           │
│  (columnar binary)       │
└──────────────────────────┘
```

## Darbība

1. **SQL lasīšana**: SQL fails tiek ielādēts atmiņā
2. **SQLcl palaišana**: Procesa palaišana ar Oracle savienojumu
3. **Sesijas konfigurācija**:
   - `SET SQLFORMAT JSON` JSON eksportam
   - `SET NLS_NUMERIC_CHARACTERS='.,';` lokalizācijas problēmu novēršanai
4. **Vaicājuma izpilde**: SQL vaicājums tiek nosūtīts caur stdin
5. **Izvades tvērums**: Pilnīga JSON stdout lasīšana
6. **JSON ekstrakcija**: Struktūras `{"results":[{"items":[...]}]}` izolēšana
7. **Shēmas secinājums**: Vortex shēma tiek automātiski secinēta no pirmā ieraksta
8. **Ierakstu konvertēšana**: Katrs JSON objekts tiek pārveidots Vortex kolonnās
9. **Faila rakstīšana**: Binārs Vortex fails tiek izveidots ar Tokio sesiju

## Atbalstītie datu tipi

JSON tipu konvertēšana uz Vortex tipiem notiek automātiski:

| JSON tips | Vortex tips | Nullable | Piezīmes |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Secinēts kā nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (vesels skaitlis) | `Primitive(I64)` | ✅ | Atpazīts ar `is_f64() == false` |
| `number` (peldošais) | `Primitive(F64)` | ✅ | Atpazīts ar `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serializēts kā JSON virkne |
| `object` | `Utf8` | ✅ | Serializēts kā JSON virkne |

**Piezīme**: Visi tipi ir nullable Oracle NULL vērtību apstrādei.

## Žurnāli un atkļūdošana

Lietotne izmanto `tracing` žurnāliem. Ziņojumi tiek parādīti stderr ar žurnāla līmeni.

Žurnāli ietver:
- Oracle savienojums
- Apstrādāto ierakstu skaits
- Secinētā shēma
- Kļūdas un brīdinājumi

## Ģenerēto Vortex failu pārbaude

Ģenerēto failu pārbaudei izmantojiet `vx` rīku:

```bash
# vx instalēšana (Vortex CLI rīks)
cargo install vortex-vx

# Vortex faila pārlūkošana
vx browse output.vortex

# Metadatu attēlošana
vx info output.vortex
```

## Ierobežojumi un apsvērumi

- **Sarežģīti tipi**: Ligzdoti JSON objekti un masīvi tiek serializēti virknēs
- **Buferis atmiņā**: Ieraksti pašlaik tiek buferēti pirms rakstīšanas (iespējama turpmāka optimizācija)
- **Fiksēta shēma**: Secinēta tikai no pirmā ieraksta (nākamajiem ierakstiem jāatbilst)
- **Drošība**: Parole tiek nodota kā CLI arguments (redzama ar `ps`). Izmantojiet vides mainīgos ražošanā.

## Izstrāde

### Build debug režīmā

```bash
cargo build
```

### Build release režīmā

```bash
cargo build --release
```

Binārs būs `target/release/oracle2vortex` (~46 MB release režīmā).

### Testi

```bash
cargo test
```

### Manuālie testi

Testa faili ar akreditācijas datiem ir `tests_local/` (gitignored):

```bash
# Testa vaicājumu izveidošana
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Izpilde
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

Šis projekts ir licencēts saskaņā ar EUPL-1.2 (European Union Public Licence v. 1.2).

**SVARĪGI - Komerciālās izmantošanas ierobežojums:**  
Šīs programmatūras komerciālā izmantošana ir aizliegta bez autora iepriekšējas rakstiskas piekrišanas.  
Komerciālās licences pieprasījumiem sazinieties: **oracle2vortex@amilto.com**

Skatiet [LICENSE](LICENSE) failu pilnam licences tekstam.

## Autors

**William Gacquer**  
Kontakts: oracle2vortex@amilto.com

## Testu vēsture

Projekts validēts uz Oracle ražošanas datubāzes:

- ✅ **Vienkāršs tests**: 10 ieraksti, 3 kolonnas → 5.5 KB
- ✅ **Sarežģīts tests**: 100 ieraksti, 417 kolonnas → 1.3 MB
- ✅ **Validācija**: Faili lasāmi ar `vx browse` (Vortex v0.58)

## Projekta struktūra

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Šis fails
├── IMPLEMENTATION.md       # Tehniskā dokumentācija
├── .gitignore             # Izslēdz tests_local/ un akreditācijas datus
├── src/
│   ├── main.rs            # Entry point ar tokio runtime
│   ├── cli.rs             # Clap argumentu apstrāde
│   ├── sqlcl.rs           # SQLcl process ar CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Konvertēšana JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Pilnīga orķestrācija
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Piemēra vaicājums
└── tests_local/           # Testi ar akreditācijas datiem (gitignored)
```

## Galvenās atkarības

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Resursi

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
