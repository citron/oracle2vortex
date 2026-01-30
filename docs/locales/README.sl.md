# oracle2vortex

CLI aplikacija za izvoz Oracle tabel v format Vortex preko SQLcl s pretočnim JSON.

## Opis

`oracle2vortex` omogoča izvoz podatkov iz Oracle z uporabo:
- **SQLcl** za povezavo in izvoren JSON izvoz
- **Pretočno procesiranje** za obdelavo podatkov sproti brez čakanja na konec izvoza
- **Samodejno pretvorbo** v stolpčni format Vortex z izpeljevanjem sheme

✅ **Projekt zaključen in preizkušen v produkciji** - Potrjen s tabelo 417 stolpcev na pravi bazi podatkov.

## Predpogoji

- **Rust nightly** (zahtevano za Vortex crate-e)
- **SQLcl** nameščen (ali navedite pot z `--sqlcl-path`)
- Dostopna Oracle podatkovna baza

### Namestitev Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Namestitev SQLcl

Prenesite SQLcl z: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Ali na Linuxu:
```bash
# Primer namestitve v /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Namestitev

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Izvedljiva datoteka bo na voljo v `target/release/oracle2vortex`.

## Uporaba

### Osnovna sintaksa

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

| Možnost | Kratka | Opis | Privzeto |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Pot do SQL datoteke, ki vsebuje poizvedbo | (obvezno) |
| `--output` | `-o` | Pot do izhodne Vortex datoteke | (obvezno) |
| `--host` | | Oracle gostitelj | (obvezno) |
| `--port` | | Oracle vrata | 1521 |
| `--user` | `-u` | Oracle uporabnik | (obvezno) |
| `--password` | `-p` | Oracle geslo | (obvezno) |
| `--sid` | | Oracle SID ali ime storitve | (obvezno) |
| `--sqlcl-path` | | Pot do izvedljive datoteke SQLcl | `sql` |
| `--auto-batch-rows` | | Število vrstic na paket (0 = onemogočeno) | 0 |

### Samodejno paketiranje (velike tabele)

Za obdelavo tabel z milijoni ali milijardami vrstic s konstantno uporabo pomnilnika uporabite možnost `--auto-batch-rows`:

```bash
# Obdelava v paketih po 50000 vrstic
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

**Kako deluje:**
1. Samodejno ovije vašo poizvedbo z `OFFSET/FETCH`
2. Izvede SQLcl večkrat (enkrat na paket)
3. Zbere vse rezultate v pomnilnik
4. Zapiše eno Vortex datoteko, ki vsebuje vse podatke

**Omejitve:**
- Zahteva Oracle 12c+ (sintaksa OFFSET/FETCH)
- Vaša poizvedba NE sme že vsebovati OFFSET/FETCH ali ROWNUM
- Priporočljivo: dodajte ORDER BY za konsistentno zaporedje

**Pomnilnik:** S samodejnim paketiranjem, uporabljen pomnilnik = velikost paketa × 2 (JSON + Vortex)  
Primer: 50000 vrstic × 1 KB = 100 MB na paket (namesto nalaganja cele tabele)

**Glej tudi:** `BATCH_PROCESSING.md` in `README_LARGE_DATASETS.md` za več podrobnosti.

### Primer z SQL datoteko

Ustvarite datoteko `query.sql`:

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

Nato izvedite:

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

## Arhitektura

```
┌─────────────┐
│  SQL        │
│  datoteka   │
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
│  .vortex datoteka        │
│  (columnar binary)       │
└──────────────────────────┘
```

## Delovanje

1. **Branje SQL**: SQL datoteka se naloži v pomnilnik
2. **Zagon SQLcl**: Zagon procesa z Oracle povezavo
3. **Konfiguracija seje**:
   - `SET SQLFORMAT JSON` za JSON izvoz
   - `SET NLS_NUMERIC_CHARACTERS='.,';` za izogibanje težavam z locale
4. **Izvajanje poizvedbe**: SQL poizvedba se pošlje prek stdin
5. **Zajem izhoda**: Popolno branje JSON stdout
6. **Ekstrakcija JSON**: Izolacija strukture `{"results":[{"items":[...]}]}`
7. **Izpeljevanje sheme**: Vortex shema se samodejno izpelje iz prvega zapisa
8. **Pretvorba zapisov**: Vsak JSON objekt se pretvori v Vortex stolpce
9. **Zapis datoteke**: Binarna Vortex datoteka se ustvari s Tokio sejo

## Podprti tipi podatkov

Pretvorba JSON tipov v Vortex tipe poteka samodejno:

| JSON tip | Vortex tip | Nullable | Opombe |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Izpeljan kot nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (celo število) | `Primitive(I64)` | ✅ | Zaznan z `is_f64() == false` |
| `number` (decimalno) | `Primitive(F64)` | ✅ | Zaznan z `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serializiran kot JSON niz |
| `object` | `Utf8` | ✅ | Serializiran kot JSON niz |

**Opomba**: Vsi tipi so nullable za obravnavo Oracle NULL vrednosti.

## Dnevniki in razhroščevanje

Aplikacija uporablja `tracing` za dnevnike. Sporočila se prikažejo na stderr z ravnjo dnevnika.

Dnevniki vključujejo:
- Povezavo z Oracle
- Število obdelanih zapisov
- Izpeljana shema
- Napake in opozorila

## Preverjanje ustvarjenih Vortex datotek

Za preverjanje ustvarjenih datotek uporabite orodje `vx`:

```bash
# Namestitev vx (Vortex CLI orodje)
cargo install vortex-vx

# Brskanje po Vortex datoteki
vx browse output.vortex

# Prikaz metapodatkov
vx info output.vortex
```

## Omejitve in premisleki

- **Kompleksni tipi**: Gnezdeni JSON objekti in polja se serializirajo v nize
- **Medpomnilnik v pomnilniku**: Zapisi so trenutno medpomnjeni pred zapisom (možna prihodnja optimizacija)
- **Fiksna shema**: Izpeljana samo iz prvega zapisa (naslednji zapisi se morajo ujemati)
- **Varnost**: Geslo se posreduje kot CLI argument (vidno z `ps`). V produkciji uporabite okoljske spremenljivke.

## Razvoj

### Build v načinu debug

```bash
cargo build
```

### Build v načinu release

```bash
cargo build --release
```

Binarna datoteka bo v `target/release/oracle2vortex` (~46 MB v release).

### Testi

```bash
cargo test
```

### Ročni testi

Testne datoteke s poverilnicami so v `tests_local/` (gitignored):

```bash
# Ustvarjanje testnih poizvedb
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Izvajanje
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licenca

Copyright (c) 2026 William Gacquer

Ta projekt je licenciran pod EUPL-1.2 (European Union Public Licence v. 1.2).

**POMEMBNO - Omejitev komercialne uporabe:**  
Komercialna uporaba te programske opreme je prepovedana brez predhodnega pisnega soglasja avtorja.  
Za kakršne koli zahteve za komercialno licenco se obrnite: **oracle2vortex@amilto.com**

Oglejte si datoteko [LICENSE](LICENSE) za celotno besedilo licence.

## Avtorji

**William Gacquer**  
Kontakt: oracle2vortex@amilto.com

## Zgodovina testiranja

Projekt je bil potrjen na produkcijski Oracle bazi podatkov:

- ✅ **Preprost test**: 10 zapisov, 3 stolpci → 5.5 KB
- ✅ **Kompleksen test**: 100 zapisov, 417 stolpcev → 1.3 MB
- ✅ **Potrditev**: Datoteke berljive z `vx browse` (Vortex v0.58)

## Struktura projekta

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Ta datoteka
├── IMPLEMENTATION.md       # Tehnična dokumentacija
├── .gitignore             # Izključuje tests_local/ in poverilnice
├── src/
│   ├── main.rs            # Entry point s tokio runtime
│   ├── cli.rs             # Obdelava Clap argumentov
│   ├── sqlcl.rs           # SQLcl proces z CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Pretvorba JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Popolna orkestacija
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Vzorčna poizvedba
└── tests_local/           # Testi s poverilnicami (gitignored)
```

## Glavne odvisnosti

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Viri

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
