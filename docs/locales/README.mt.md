# oracle2vortex

Applikazzjoni CLI li tesporta tabelli Oracle għal format Vortex permezz ta' SQLcl b'streaming JSON.

## Deskrizzjoni

`oracle2vortex` jippermetti l-esportazzjoni ta' data Oracle billi juża:
- **SQLcl** għall-konnessjoni u esportazzjoni nattiva JSON
- **Streaming** biex tipproċessa data fil-volu mingħajr ma tistenna l-esportazzjoni titlesta
- **Konverżjoni awtomatika** għal format kolonnari Vortex b'inferenza ta' skema

✅ **Proġett komplut u ttestjat fil-produzzjoni** - Validat b'tabella ta' 417 kolonna fuq database reali.

## Prekondizzjonijiet

- **Rust nightly** (meħtieġ mill-crates Vortex)
- **SQLcl** installat (jew speċifika l-path b'`--sqlcl-path`)
- Database Oracle aċċessibbli

### Installazzjoni ta' Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Installazzjoni ta' SQLcl

Niżżel SQLcl minn: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Jew fuq Linux:
```bash
# Eżempju għall-installazzjoni f'/opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Installazzjoni

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

L-eżekwabbli jkun disponibbli f'`target/release/oracle2vortex`.

## Użu

### Sintassi bażika

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

### Għażliet

| Għażla | Qasir | Deskrizzjoni | Default |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Path għall-fajl SQL li fih il-query | (meħtieġ) |
| `--output` | `-o` | Path tal-fajl Vortex tal-output | (meħtieġ) |
| `--host` | | Host Oracle | (meħtieġ) |
| `--port` | | Port Oracle | 1521 |
| `--user` | `-u` | User Oracle | (meħtieġ) |
| `--password` | `-p` | Password Oracle | (meħtieġ) |
| `--sid` | | SID Oracle jew isem tas-servizz | (meħtieġ) |
| `--sqlcl-path` | | Path għall-eżekwabbli SQLcl | `sql` |
| `--auto-batch-rows` | | Numru ta' ringiela għal kull batch (0 = diżattivat) | 0 |
| `--skip-lobs` | | Aqbeż tipi LOB Oracle (CLOB, BLOB, NCLOB) | false |

### Auto-batching (tabelli kbar)

Biex tipproċessa tabelli b'miljuni jew biljuni ta' ringiela b'użu kostanti tal-memorja, uża l-għażla `--auto-batch-rows`:

```bash
# Ipproċessa f'batches ta' 50000 ringiela
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

**Kif jaħdem:**
1. Awtomatikament jagħlaq il-query tiegħek b'`OFFSET/FETCH`
2. Jeżegwixxi SQLcl diversi darbiet (darba għal kull batch)
3. Jiġbor ir-riżultati kollha fil-memorja
4. Jikteb fajl Vortex wieħed li fih id-data kollha

**Limitazzjonijiet:**
- Jeħtieġ Oracle 12c+ (sintassi OFFSET/FETCH)
- Il-query tiegħek M'GĦANDHIEX diġà jkun fih OFFSET/FETCH jew ROWNUM
- Rakkomandat: żid ORDER BY għal ordni konsistenti

**Memorja:** B'auto-batching, memorja użata = daqs tal-batch × 2 (JSON + Vortex)  
Eżempju: 50000 ringiela × 1 KB = 100 MB għal kull batch (minflok ma ttella' t-tabella kollha)

**Ara wkoll:** `BATCH_PROCESSING.md` u `README_LARGE_DATASETS.md` għal aktar dettalji.

### Aqbeż kolonni LOB

Tipi LOB Oracle (CLOB, BLOB, NCLOB) jistgħu jkunu kbar ħafna u jista' jkun li mhumiex meħtieġa għall-analiżi. Uża `--skip-lobs` biex teskludi huma:

```bash
# Aqbeż kolonni LOB biex tnaqqas id-daqs tal-fajl u ttejjeb il-prestazzjoni
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

**Kif jaħdem:**
- Awtomatikament jiskopri u jiffiltra kolonni li fihom data LOB
- LOBs huma identifikati bid-daqs (> 4000 karattru) jew indikaturi binari
- L-ewwel rekord irreġistrat se juri kemm kolonni ġew maqbuża
- Jnaqqas b'mod sinifikanti d-daqs tal-fajl u l-użu tal-memorja għal tabelli b'oqsma kbar ta' test/binarji

**Każijiet ta' użu:**
- Esportazzjoni ta' tabelli metadata b'oqsma ta' deskrizzjoni
- Ħidma ma' tabelli li fihom dokumenti XML jew JSON kbar
- Iffoka fuq data strutturat filwaqt li tinjora kontenut binarju
- Ottimizzazzjoni tal-prestazzjoni għal tabelli b'ħafna kolonni kbar

### Eżempju b'fajl SQL

Oħloq fajl `query.sql`:

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

Imbagħad eżegwixxi:

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

## Arkitettura

```
┌─────────────┐
│  Fajl       │
│  SQL        │
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
│  Fajl .vortex            │
│  (columnar binary)       │
└──────────────────────────┘
```

## Funzjonament

1. **Qari SQL**: Il-fajl SQL jitgħabba fil-memorja
2. **Tħaddim SQLcl**: Tħaddim tal-proċess b'konnessjoni Oracle
3. **Konfigurazzjoni tas-sessjoni**:
   - `SET SQLFORMAT JSON` għall-esportazzjoni JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` biex tevita problemi ta' locale
4. **Eżekuzzjoni tal-query**: Il-query SQL jintbagħat permezz stdin
5. **Qbid tal-output**: Qari sħiħ tal-JSON stdout
6. **Estratt JSON**: Iżolament tal-istruttura `{"results":[{"items":[...]}]}`
7. **Inferenza tal-iskema**: L-iskema Vortex tinħareġ awtomatikament mill-ewwel record
8. **Konverżjoni tar-records**: Kull oġġett JSON jiġi ttransformat f'kolonni Vortex
9. **Kitba tal-fajl**: Fajl binarju Vortex jinħoloq b'sessjoni Tokio

## Tipi ta' data appoġġati

Il-konverżjoni tat-tipi JSON għal tipi Vortex isseħħ awtomatikament:

| Tip JSON | Tip Vortex | Nullable | Noti |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Inferit bħala string nullable |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (sħiħ) | `Primitive(I64)` | ✅ | Identifikat b'`is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Identifikat b'`is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Sserjalizzat bħala string JSON |
| `object` | `Utf8` | ✅ | Sserjalizzat bħala string JSON |

**Nota**: It-tipi kollha huma nullable biex jimmaniġġaw valuri Oracle NULL.

## Logs u debugging

L-applikazzjoni tuża `tracing` għal-logs. Il-messaġġi jintwerew fuq stderr bil-livell tal-log.

Il-logs jinkludu:
- Konnessjoni ma' Oracle
- Numru ta' records ipproċessati
- Skema inferita
- Żbalji u twissijiet

## Verifika ta' fajls Vortex iġġenerati

Biex tivverifika l-fajls iġġenerati, uża l-għodda `vx`:

```bash
# Installazzjoni ta' vx (għodda Vortex CLI)
cargo install vortex-vx

# Esplora fajl Vortex
vx browse output.vortex

# Uri metadata
vx info output.vortex
```

## Limitazzjonijiet u kunsiderazzjonijiet

- **Tipi kumplessi**: Oġġetti JSON nidifikati u arrays jiġu sserjalizzati f'strings
- **Buffer fil-memorja**: Records bħalissa huma buffered qabel il-kitba (ottimizzazzjoni futura possibbli)
- **Skema fissa**: Inferita biss mill-ewwel record (records li jmiss għandhom jaqblu)
- **Sigurtà**: Il-password tgħaddi bħala argument CLI (viżibbli b'`ps`). Uża varjabbli tal-ambjent fil-produzzjoni.
- **Tipi LOB**: B'mod default, kolonni LOB (CLOB, BLOB, NCLOB) huma inklużi. Uża `--skip-lobs` biex teskludi huma għal prestazzjoni aħjar u daqsijiet tal-fajl iżgħar.

## Żvilupp

### Build f'modalità debug

```bash
cargo build
```

### Build f'modalità release

```bash
cargo build --release
```

Il-binary ikun f'`target/release/oracle2vortex` (~46 MB f'release).

### Tests

```bash
cargo test
```

### Tests manwali

Il-fajls tat-test b'kredenzjali huma f'`tests_local/` (gitignored):

```bash
# Oħloq queries tat-test
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Eżegwixxi
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Liċenzja

Copyright (c) 2026 William Gacquer

Dan il-proġett huwa liċenzjat taħt EUPL-1.2 (European Union Public Licence v. 1.2).

**IMPORTANTI - Restrizzjoni ta' użu kummerċjali:**  
L-użu kummerċjali ta' dan is-software huwa projbit mingħajr ftehim bil-miktub minn qabel mal-awtur.  
Għal kwalunkwe talba għal liċenzja kummerċjali, ikkuntattja: **oracle2vortex@amilto.com**

Ara l-fajl [LICENSE](LICENSE) għat-test sħiħ tal-liċenzja.

## Awtur

**William Gacquer**  
Kuntatt: oracle2vortex@amilto.com

## Storja tat-tests

Il-proġett ġie validat fuq database Oracle tal-produzzjoni:

- ✅ **Test sempliċi**: 10 records, 3 kolonni → 5.5 KB
- ✅ **Test kumpless**: 100 records, 417 kolonni → 1.3 MB
- ✅ **Validazzjoni**: Fajls leġġibbli b'`vx browse` (Vortex v0.58)

## Struttura tal-proġett

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Dan il-fajl
├── IMPLEMENTATION.md       # Dokumentazzjoni teknika
├── .gitignore             # Jeskludi tests_local/ u kredenzjali
├── src/
│   ├── main.rs            # Entry point b'runtime tokio
│   ├── cli.rs             # Parsing ta' argumenti Clap
│   ├── sqlcl.rs           # Proċess SQLcl b'CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Konverżjoni JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Orkestrazzjoni kompleta
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Query ta' eżempju
└── tests_local/           # Tests b'kredenzjali (gitignored)
```

## Dipendenzi prinċipali

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Riżorsi

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
