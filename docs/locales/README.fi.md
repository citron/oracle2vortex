# oracle2vortex

CLI-sovellus, joka vie Oracle-tauluja Vortex-muotoon SQLcl:n kautta JSON-suoratoistoilla.

## Kuvaus

`oracle2vortex` mahdollistaa Oracle-datan viennin käyttäen:
- **SQLcl** yhteyteen ja natiivia JSON-vientiä
- **Suoratoisto** datan käsittelyyn lennossa ilman viennin päättymistä
- **Automaattinen muunnos** Vortex-sarakkeelliseen muotoon skeeman päättelyllä

✅ **Projekti valmis ja testattu tuotannossa** - Vahvistettu 417 sarakkeen taululla oikeassa tietokannassa.

## Edellytykset

- **Rust nightly** (vaaditaan Vortex crateille)
- **SQLcl** asennettuna (tai määritä polku `--sqlcl-path` -optiolla)
- Käytettävissä oleva Oracle-tietokanta

### Rust nightly -asennus

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### SQLcl-asennus

Lataa SQLcl osoitteesta: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Tai Linuxissa:
```bash
# Esimerkki asennuksesta kansioon /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Asennus

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Suoritettava tiedosto on saatavilla polusta `target/release/oracle2vortex`.

## Käyttö

### Perussyntaksi

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

### Vaihtoehdot

| Vaihtoehto | Lyhyt | Kuvaus | Oletus |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Kyselyn sisältävän SQL-tiedoston polku | (vaaditaan) |
| `--output` | `-o` | Tulostettavan Vortex-tiedoston polku | (vaaditaan) |
| `--host` | | Oracle-palvelin | (vaaditaan) |
| `--port` | | Oracle-portti | 1521 |
| `--user` | `-u` | Oracle-käyttäjä | (vaaditaan) |
| `--password` | `-p` | Oracle-salasana | (vaaditaan) |
| `--sid` | | Oracle SID tai palvelunimi | (vaaditaan) |
| `--sqlcl-path` | | SQLcl-suoritettavan polku | `sql` |
| `--auto-batch-rows` | | Rivien määrä erässä (0 = pois käytöstä) | 0 |

### Automaattinen eräkäsittely (suuret taulut)

Miljoonien tai miljardien rivien taulujen käsittelyyn vakiomuistinkäytöllä, käytä `--auto-batch-rows` -optiota:

```bash
# Käsittele 50000 rivin erissä
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

**Toimintaperiaate:**
1. Käärii kyselyn automaattisesti `OFFSET/FETCH` -lauseella
2. Suorittaa SQLcl:n useita kertoja (kerran per erä)
3. Kerää kaikki tulokset muistiin
4. Kirjoittaa yhden Vortex-tiedoston, joka sisältää kaiken datan

**Rajoitukset:**
- Vaatii Oracle 12c+ (OFFSET/FETCH syntaksi)
- Kyselysi EI SAA jo sisältää OFFSET/FETCH tai ROWNUM
- Suositus: lisää ORDER BY yhtenäisen järjestyksen varmistamiseksi

**Muisti:** Automaattisella eräkäsittelyllä, käytetty muisti = erän koko × 2 (JSON + Vortex)  
Esimerkki: 50000 riviä × 1 KB = 100 MB per erä (koko taulun lataamisen sijaan)

**Katso myös:** `BATCH_PROCESSING.md` ja `README_LARGE_DATASETS.md` lisätietoja varten.

### Esimerkki SQL-tiedostolla

Luo tiedosto `query.sql`:

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

Suorita sitten:

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

## Arkkitehtuuri

```
┌─────────────┐
│  SQL-       │
│  tiedosto   │
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
│  .vortex-tiedosto        │
│  (columnar binary)       │
└──────────────────────────┘
```

## Toiminta

1. **SQL-luku**: SQL-tiedosto ladataan muistiin
2. **SQLcl-käynnistys**: Prosessin käynnistys Oracle-yhteydellä
3. **Istunnon konfigurointi**:
   - `SET SQLFORMAT JSON` JSON-vientiä varten
   - `SET NLS_NUMERIC_CHARACTERS='.,';` locale-ongelmien välttämiseksi
4. **Kyselyn suoritus**: SQL-kysely lähetetään stdin:n kautta
5. **Tulosteen kaappaus**: JSON stdout:n täydellinen lukeminen
6. **JSON-purku**: Rakenteen `{"results":[{"items":[...]}]}` eristäminen
7. **Skeeman päättely**: Vortex-skeema johdetaan automaattisesti ensimmäisestä tietueesta
8. **Tietueiden muunnos**: Jokainen JSON-objekti muunnetaan Vortex-sarakkeiksi
9. **Tiedoston kirjoitus**: Binäärinen Vortex-tiedosto luodaan Tokio-istunnolla

## Tuetut tietotyypit

JSON-tyyppien muunnos Vortex-tyypeiksi tapahtuu automaattisesti:

| JSON-tyyppi | Vortex-tyyppi | Nullable | Huomiot |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Päätelty nullable string-tyypiksi |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (kokonaisluku) | `Primitive(I64)` | ✅ | Havaittu kun `is_f64() == false` |
| `number` (liukuluku) | `Primitive(F64)` | ✅ | Havaittu kun `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Sarjoitettu JSON-merkkijonoksi |
| `object` | `Utf8` | ✅ | Sarjoitettu JSON-merkkijonoksi |

**Huomio**: Kaikki tyypit ovat nullable Oracle NULL -arvojen käsittelyä varten.

## Lokit ja virheenkorjaus

Sovellus käyttää `tracing` -kirjastoa lokeihin. Viestit näytetään stderr:ssä lokitasoineen.

Lokit sisältävät:
- Oracle-yhteys
- Käsiteltyjen tietueiden määrä
- Päätelty skeema
- Virheet ja varoitukset

## Luotujen Vortex-tiedostojen tarkistus

Luotujen tiedostojen tarkistamiseen käytä `vx`-työkalua:

```bash
# vx:n asennus (Vortex CLI -työkalu)
cargo install vortex-vx

# Vortex-tiedoston selaus
vx browse output.vortex

# Metatietojen näyttäminen
vx info output.vortex
```

## Rajoitukset ja huomiot

- **Monimutkaiset tyypit**: Sisäkkäiset JSON-objektit ja taulukot sarjoitetaan merkkijonoiksi
- **Muistipuskuri**: Tietueet puskuroidaan tällä hetkellä ennen kirjoitusta (tulevaa optimointia mahdollista)
- **Kiinteä skeema**: Päätelty vain ensimmäisestä tietueesta (seuraavien tietueiden on vastattava)
- **Turvallisuus**: Salasana välitetään CLI-argumenttina (näkyvissä `ps`:llä). Käytä ympäristömuuttujia tuotannossa.

## Kehitys

### Käännös debug-tilassa

```bash
cargo build
```

### Käännös release-tilassa

```bash
cargo build --release
```

Binääri on polulla `target/release/oracle2vortex` (~46 MB release-tilassa).

### Testit

```bash
cargo test
```

### Manuaaliset testit

Testitiedostot tunnistetietoineen ovat kansiossa `tests_local/` (gitignored):

```bash
# Luo testikyselyjä
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Suorita
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Lisenssi

Copyright (c) 2026 William Gacquer

Tämä projekti on lisensoitu EUPL-1.2 -lisenssillä (European Union Public Licence v. 1.2).

**TÄRKEÄÄ - Kaupallisen käytön rajoitus:**  
Tämän ohjelmiston kaupallinen käyttö on kielletty ilman tekijän kirjallista ennakkosuostumusta.  
Kaupallista lisenssiä koskevat pyynnöt: **oracle2vortex@amilto.com**

Katso [LICENSE](LICENSE) -tiedosto lisenssin täydellisestä tekstistä.

## Tekijä

**William Gacquer**  
Yhteystieto: oracle2vortex@amilto.com

## Testihistoria

Projekti on validoitu Oracle-tuotantotietokannassa:

- ✅ **Yksinkertainen testi**: 10 tietuetta, 3 saraketta → 5.5 KB
- ✅ **Monimutkainen testi**: 100 tietuetta, 417 saraketta → 1.3 MB
- ✅ **Validointi**: Tiedostot luettavissa `vx browse` -työkalulla (Vortex v0.58)

## Projektin rakenne

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Tämä tiedosto
├── IMPLEMENTATION.md       # Tekninen dokumentaatio
├── .gitignore             # Sulkee pois tests_local/ ja tunnistetiedot
├── src/
│   ├── main.rs            # Entry point tokio-ajoympäristöllä
│   ├── cli.rs             # Clap-argumenttien jäsennys
│   ├── sqlcl.rs           # SQLcl-prosessi CONNECT:llä
│   ├── json_stream.rs     # Jäsennin {"results":[...]}
│   ├── vortex_writer.rs   # Muunnos JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Täydellinen orkestrointi
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Esimerkkikysely
└── tests_local/           # Testit tunnistetiedoilla (gitignored)
```

## Tärkeimmät riippuvuudet

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Resurssit

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
