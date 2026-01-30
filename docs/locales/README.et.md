# oracle2vortex

CLI rakendus Oracle tabelite ekspordiks Vortex formaati SQLcl kaudu JSON voogedastusega.

## Kirjeldus

`oracle2vortex` võimaldab Oracle andmete eksporti kasutades:
- **SQLcl** ühenduseks ja kodupäraseks JSON ekspordiks
- **Voogedastus** andmete töötlemiseks lennult ilma ekspordi lõppu ootamata
- **Automaatne teisendus** veeru-põhisesse Vortex formaati skeemi järeldamisega

✅ **Projekt valmis ja testitud tootmises** - Valideeritud 417 veeru tabeliga päris andmebaasil.

## Eeldused

- **Rust nightly** (nõutav Vortex crate-de jaoks)
- **SQLcl** installitud (või määrake tee `--sqlcl-path` valikuga)
- Kättesaadav Oracle andmebaas

### Rust nightly installimine

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### SQLcl installimine

Laadige SQLcl alla: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Või Linuxis:
```bash
# Näide installimiseks /opt/oracle/sqlcl/ kausta
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Installimine

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Käivitatav fail on saadaval asukohas `target/release/oracle2vortex`.

## Kasutamine

### Põhisüntaks

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

### Valikud

| Valik | Lühike | Kirjeldus | Vaikimisi |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Tee SQL failini, mis sisaldab päringut | (kohustuslik) |
| `--output` | `-o` | Väljundi Vortex faili tee | (kohustuslik) |
| `--host` | | Oracle host | (kohustuslik) |
| `--port` | | Oracle port | 1521 |
| `--user` | `-u` | Oracle kasutaja | (kohustuslik) |
| `--password` | `-p` | Oracle parool | (kohustuslik) |
| `--sid` | | Oracle SID või teenuse nimi | (kohustuslik) |
| `--sqlcl-path` | | Tee SQLcl käivitatava failini | `sql` |
| `--auto-batch-rows` | | Ridade arv partii kohta (0 = välja lülitatud) | 0 |
| `--skip-lobs` | | Jäta vahele Oracle LOB tüübid (CLOB, BLOB, NCLOB) | false |

### Automaatne partiitöötlus (suured tabelid)

Miljonite või miljardite ridadega tabelite töötlemiseks konstantse mälukasutusega kasutage valikut `--auto-batch-rows`:

```bash
# Töötlemine 50000 rea partiides
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

**Kuidas see toimib:**
1. Mähib automaatselt teie päringu `OFFSET/FETCH`-iga
2. Käivitab SQLcl mitu korda (üks kord partii kohta)
3. Kogub kõik tulemused mällu
4. Kirjutab ühe Vortex faili, mis sisaldab kõiki andmeid

**Piirangud:**
- Nõuab Oracle 12c+ (OFFSET/FETCH süntaks)
- Teie päring EI tohi juba sisaldada OFFSET/FETCH või ROWNUM
- Soovitatud: lisage ORDER BY järjekindla järjestuse jaoks

**Mälu:** Automaatse partiitöötlusega, kasutatud mälu = partii suurus × 2 (JSON + Vortex)  
Näide: 50000 rida × 1 KB = 100 MB partii kohta (kogu tabeli laadimise asemel)

**Vaata ka:** `BATCH_PROCESSING.md` ja `README_LARGE_DATASETS.md` rohkemate detailide jaoks.

### LOB veergude vahelejätmine

Oracle LOB tüübid (CLOB, BLOB, NCLOB) võivad olla väga suured ja ei pruugi olla analüüsiks vajalikud. Kasutage `--skip-lobs` nende välistamiseks:

```bash
# Jäta vahele LOB veerud faili suuruse vähendamiseks ja jõudluse parandamiseks
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

**Kuidas see toimib:**
- Tuvastab ja filtreerib automaatselt LOB andmeid sisaldavad veerud
- LOB-id tuvastatakse suuruse (> 4000 tähemärki) või binaarsetest indikaatoritest
- Esimene logitud kirje näitab, mitu veergu vahele jäeti
- Vähendab oluliselt faili suurust ja mälukasutust suurte teksti-/binaarväljadega tabelite puhul

**Kasutusjuhud:**
- Metaandmete tabelite eksportimine kirjeldusväljadega
- Töötamine XML-i või suuri JSON-dokumente sisaldavate tabelitega
- Fookus struktureeritud andmetel, ignoreerides binaarset sisu
- Jõudluse optimeerimine paljude suurte veergudega tabelite jaoks

### Näide SQL failiga

Looge fail `query.sql`:

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

Seejärel käivitage:

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

## Arhitektuur

```
┌─────────────┐
│  SQL        │
│  fail       │
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
│  .vortex fail            │
│  (columnar binary)       │
└──────────────────────────┘
```

## Toimimine

1. **SQL lugemine**: SQL fail laaditakse mällu
2. **SQLcl käivitamine**: Protsessi käivitamine Oracle ühendusega
3. **Seansi konfigureerimine**:
   - `SET SQLFORMAT JSON` JSON ekspordi jaoks
   - `SET NLS_NUMERIC_CHARACTERS='.,';` locale probleemide vältimiseks
4. **Päringu täitmine**: SQL päring saadetakse läbi stdin
5. **Väljundi haaramine**: JSON stdout täielik lugemine
6. **JSON ekstraheerimine**: Struktuuri `{"results":[{"items":[...]}]}` isoleerimine
7. **Skeemi järeldamine**: Vortex skeem järeldatakse automaatselt esimesest kirjest
8. **Kirjete teisendamine**: Iga JSON objekt teisendatakse Vortex veergudeks
9. **Faili kirjutamine**: Binaarne Vortex fail luuakse Tokio seansiga

## Toetatud andmetüübid

JSON tüüpide teisendamine Vortex tüüpideks toimub automaatselt:

| JSON tüüp | Vortex tüüp | Nullable | Märkused |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Järeldatud kui nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (täisarv) | `Primitive(I64)` | ✅ | Tuvastatud kui `is_f64() == false` |
| `number` (ujukomaarv) | `Primitive(F64)` | ✅ | Tuvastatud kui `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serialiseeritud JSON stringina |
| `object` | `Utf8` | ✅ | Serialiseeritud JSON stringina |

**Märkus**: Kõik tüübid on nullable Oracle NULL väärtuste käsitlemiseks.

## Logid ja silumine

Rakendus kasutab `tracing` logide jaoks. Sõnumid kuvatakse stderr-is koos logi tasemega.

Logid sisaldavad:
- Oracle ühendus
- Töödeldud kirjete arv
- Järeldatud skeem
- Vead ja hoiatused

## Genereeritud Vortex failide kontrollimine

Genereeritud failide kontrollimiseks kasutage `vx` tööriista:

```bash
# vx installimine (Vortex CLI tööriist)
cargo install vortex-vx

# Vortex faili sirvimine
vx browse output.vortex

# Metaandmete kuvamine
vx info output.vortex
```

## Piirangud ja kaalutlused

- **Keerukad tüübid**: Pesastatud JSON objektid ja massiivid serialiseeritakse stringideks
- **Puhver mälus**: Kirjed puhverdatakse praegu enne kirjutamist (tulevane optimeerimine võimalik)
- **Fikseeritud skeem**: Järeldatud ainult esimesest kirjest (järgmised kirjed peavad vastama)
- **Turvalisus**: Parool edastatakse CLI argumendina (nähtav `ps`-ga). Kasutage tootmises keskkonnamuutujaid.
- **LOB tüübid**: Vaikimisi on LOB veerud (CLOB, BLOB, NCLOB) kaasatud. Kasutage `--skip-lobs` nende välistamiseks parema jõudluse ja väiksemate failide suuruse saavutamiseks.

## Arendus

### Build debug režiimis

```bash
cargo build
```

### Build release režiimis

```bash
cargo build --release
```

Binaarne fail on asukohas `target/release/oracle2vortex` (~46 MB release-s).

### Testid

```bash
cargo test
```

### Manuaalsed testid

Testfailid volitustega on kaustas `tests_local/` (gitignored):

```bash
# Testpäringute loomine
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Käivitamine
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Litsents

Copyright (c) 2026 William Gacquer

See projekt on litsentsitud EUPL-1.2 (European Union Public Licence v. 1.2) all.

**OLULINE - Kaubandusliku kasutuse piirang:**  
Selle tarkvara kaubanduslik kasutamine on keelatud ilma autori eelneva kirjaliku nõusolekuta.  
Kaubandusliku litsentsi taotluste kohta võtke ühendust: **oracle2vortex@amilto.com**

Vaadake [LICENSE](LICENSE) faili litsentsi täieliku teksti jaoks.

## Autor

**William Gacquer**  
Kontakt: oracle2vortex@amilto.com

## Testide ajalugu

Projekt on valideeritud Oracle tootmisandmebaasil:

- ✅ **Lihtne test**: 10 kirjet, 3 veergu → 5.5 KB
- ✅ **Keeruline test**: 100 kirjet, 417 veergu → 1.3 MB
- ✅ **Valideerimine**: Failid loetavad `vx browse`-ga (Vortex v0.58)

## Projekti struktuur

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # See fail
├── IMPLEMENTATION.md       # Tehniline dokumentatsioon
├── .gitignore             # Välistab tests_local/ ja volitused
├── src/
│   ├── main.rs            # Entry point tokio runtime-ga
│   ├── cli.rs             # Clap argumentide töötlemine
│   ├── sqlcl.rs           # SQLcl protsess CONNECT-iga
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Teisendus JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Täielik orkestratsioon
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Näidispäring
└── tests_local/           # Testid volitustega (gitignored)
```

## Peamised sõltuvused

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Ressursid

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
