# oracle2vortex

Feidhmchlár CLI a easpórtálann táblaí Oracle go formáid Vortex trí SQLcl le sruthú JSON.

## Cur Síos

Ceadaíonn `oracle2vortex` easpórtáil sonraí Oracle ag úsáid:
- **SQLcl** don nasc agus easpórtáil dhúchasach JSON
- **Sruthú** chun sonraí a phróiseáil ar an eitilt gan fanacht le críoch an easpórtála
- **Tiontú uathoibríoch** go formáid cholúnach Vortex le hinfheidhmiú scéime

✅ **Tionscadal críochnaithe agus tástáilte i dtáirgeadh** - Bailíochtaithe le tábla 417 colún ar fhíor-bhunachar sonraí.

## Réamhriachtanais

- **Rust nightly** (ag teastáil ó chráta Vortex)
- **SQLcl** suiteáilte (nó sonraigh an cosán le `--sqlcl-path`)
- Bunachar sonraí Oracle inrochtana

### Suiteáil Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Suiteáil SQLcl

Íoslódáil SQLcl ó: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Nó ar Linux:
```bash
# Sampla le haghaidh suiteála i /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Suiteáil

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Beidh an comhad inrite ar fáil i `target/release/oracle2vortex`.

## Úsáid

### Comhréir bhunúsach

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

### Roghanna

| Rogha | Gearr | Cur Síos | Réamhshocrú |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Cosán chuig an gcomhad SQL leis an gceist | (riachtanach) |
| `--output` | `-o` | Cosán an chomhaid Vortex aschuir | (riachtanach) |
| `--host` | | Óstach Oracle | (riachtanach) |
| `--port` | | Port Oracle | 1521 |
| `--user` | `-u` | Úsáideoir Oracle | (riachtanach) |
| `--password` | `-p` | Pasfhocal Oracle | (riachtanach) |
| `--sid` | | SID Oracle nó ainm seirbhíse | (riachtanach) |
| `--sqlcl-path` | | Cosán chuig an gcomhad inrite SQLcl | `sql` |
| `--auto-batch-rows` | | Líon na línte in aghaidh an bhaisc (0 = díchumasaithe) | 0 |

### Uath-bhaiscíocht (táblaí móra)

Chun táblaí a phróiseáil le milliúin nó billiúin línte le húsáid seasmhach cuimhne, úsáid an rogha `--auto-batch-rows`:

```bash
# Próiseáil i mbaisc de 50000 líne
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

**Conas a oibríonn sé:**
1. Fillteann sé do cheist go huathoibríoch le `OFFSET/FETCH`
2. Ritheann sé SQLcl roinnt uaireanta (uair amháin in aghaidh an bhaisc)
3. Bailíonn sé na torthaí go léir sa chuimhne
4. Scríobhann sé comhad Vortex amháin leis na sonraí go léir

**Teorainneacha:**
- Teastaíonn Oracle 12c+ (comhréir OFFSET/FETCH)
- NÍ MÓR do cheist OFFSET/FETCH nó ROWNUM a bheith ann cheana
- Molta: cuir ORDER BY leis le haghaidh ord comhsheasmhach

**Cuimhne:** Le uath-bhaiscíocht, cuimhne úsáidte = méid an bhaisc × 2 (JSON + Vortex)  
Sampla: 50000 líne × 1 KB = 100 MB in aghaidh an bhaisc (in ionad an tábla iomlán a lódáil)

**Féach freisin:** `BATCH_PROCESSING.md` agus `README_LARGE_DATASETS.md` le haghaidh tuilleadh sonraí.

### Sampla le comhad SQL

Cruthaigh comhad `query.sql`:

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

Ansin rith:

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

## Ailtireacht

```
┌─────────────┐
│  Comhad     │
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
│  Comhad .vortex          │
│  (columnar binary)       │
└──────────────────────────┘
```

## Oibriú

1. **Léamh SQL**: Lódáltar an comhad SQL sa chuimhne
2. **Tosú SQLcl**: Tosú an phróisis le nasc Oracle
3. **Cumraíocht seisiúin**:
   - `SET SQLFORMAT JSON` le haghaidh easpórtála JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` chun fadhbanna logchaighdeáin a sheachaint
4. **Forghníomhú ceiste**: Seoltar an cheist SQL trí stdin
5. **Gabháil aschuir**: Léamh iomlán de stdout JSON
6. **Eastóscadh JSON**: Leithlisiú an struchtúir `{"results":[{"items":[...]}]}`
7. **Infheidhmiú scéime**: Aschruthaítear scéim Vortex go huathoibríoch ón gcéad taifead
8. **Tiontú taifead**: Tiontaítear gach réad JSON go colúin Vortex
9. **Scríobh comhaid**: Cruthaítear comhad dénártha Vortex le seisiún Tokio

## Cineálacha sonraí tacaithe

Tarlaíonn tiontú cineálacha JSON go cineálacha Vortex go huathoibríoch:

| Cineál JSON | Cineál Vortex | Nullable | Nótaí |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Aschruthaíthe mar theaghrán nullable |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (slánuimhir) | `Primitive(I64)` | ✅ | Braithe le `is_f64() == false` |
| `number` (snámhphointe) | `Primitive(F64)` | ✅ | Braithe le `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Srathaithe mar theaghrán JSON |
| `object` | `Utf8` | ✅ | Srathaithe mar theaghrán JSON |

**Nóta**: Tá gach cineál nullable chun luachanna Oracle NULL a láimhseáil.

## Logaí agus dífhabhtú

Úsáideann an feidhmchlár `tracing` le haghaidh logaí. Taispeántar teachtaireachtaí ar stderr le leibhéal an loga.

Áirítear sna logaí:
- Nasc le Oracle
- Líon na dtaifead próiseáilte
- Scéim aschruthaíthe
- Earráidí agus rabhaidh

## Fíorú comhad Vortex ginte

Chun comhaid ginte a fhíorú, úsáid an uirlis `vx`:

```bash
# Suiteáil vx (uirlis Vortex CLI)
cargo install vortex-vx

# Brabhsáil comhad Vortex
vx browse output.vortex

# Taispeáin meiteashonraí
vx info output.vortex
```

## Teorainneacha agus breithnithe

- **Cineálacha casta**: Srathaitear réada JSON neadaithe agus eagair i dteaghráin
- **Maolán sa chuimhne**: Tá taifid i maolán faoi láthair roimh scríobh (optamú sa todhchaí indéanta)
- **Scéim seasta**: Aschruthaíthe ón gcéad taifead amháin (caithfidh taifid ina dhiaidh sin a bheith ag teacht)
- **Slándáil**: Pasáiltear an pasfhocal mar argóint CLI (infheicthe le `ps`). Úsáid athróga timpeallachta i dtáirgeadh.

## Forbairt

### Tógáil i mód dífhabhtaithe

```bash
cargo build
```

### Tógáil i mód scaoileadh

```bash
cargo build --release
```

Beidh an dénártha i `target/release/oracle2vortex` (~46 MB i scaoileadh).

### Tástálacha

```bash
cargo test
```

### Tástálacha láimhe

Tá comhaid tástála le dintiúir i `tests_local/` (gitignored):

```bash
# Cruthaigh ceisteanna tástála
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Rith
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Ceadúnas

Copyright (c) 2026 William Gacquer

Tá an tionscadal seo faoi cheadúnas EUPL-1.2 (European Union Public Licence v. 1.2).

**TÁBHACHTACH - Srianta úsáide tráchtála:**  
Tá úsáid tráchtála an bhogearraí seo coiscthe gan comhaontú scríofa réamh-údaraithe leis an údar.  
Le haghaidh aon iarratas ar cheadúnas tráchtála, déan teagmháil: **oracle2vortex@amilto.com**

Féach ar an gcomhad [LICENSE](LICENSE) le haghaidh téacs iomlán an cheadúnais.

## Údar

**William Gacquer**  
Teagmháil: oracle2vortex@amilto.com

## Stair tástála

Bailíodh an tionscadal ar bhunachar sonraí Oracle táirgeachta:

- ✅ **Tástáil shimplí**: 10 dtaifead, 3 cholún → 5.5 KB
- ✅ **Tástáil chasta**: 100 taifead, 417 colún → 1.3 MB
- ✅ **Bailíochtú**: Comhaid inléite le `vx browse` (Vortex v0.58)

## Struchtúr an tionscadail

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # An comhad seo
├── IMPLEMENTATION.md       # Doiciméadacht theicniúil
├── .gitignore             # Eisiamh tests_local/ agus dintiúir
├── src/
│   ├── main.rs            # Entry point le runtime tokio
│   ├── cli.rs             # Parsáil argóintí Clap
│   ├── sqlcl.rs           # Próiseas SQLcl le CONNECT
│   ├── json_stream.rs     # Parsálaí {"results":[...]}
│   ├── vortex_writer.rs   # Tiontú JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Orfhídheadhaint iomlán
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Ceist shamplach
└── tests_local/           # Tástálacha le dintiúir (gitignored)
```

## Spleáchais phríomha

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Acmhainní

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
