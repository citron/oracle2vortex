# oracle2vortex

CLI aplikacija koja izvozi Oracle tablice u Vortex format putem SQLcl-a sa JSON streamingom.

## Opis

`oracle2vortex` omogućava izvoz Oracle podataka koristeći:
- **SQLcl** za povezivanje i nativni JSON izvoz
- **Streaming** za obradu podataka u hodu bez čekanja završetka izvoza
- **Automatska konverzija** u stupčasti Vortex format s inferencijom sheme

✅ **Projekt završen i testiran u produkciji** - Validiran s tablicom od 417 stupaca na pravoj bazi podataka.

## Preduvjeti

- **Rust nightly** (potrebno za Vortex crate-ove)
- **SQLcl** instaliran (ili navedite putanju s `--sqlcl-path`)
- Dostupna Oracle baza podataka

### Instalacija Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Instalacija SQLcl

Preuzmite SQLcl s: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Ili na Linuxu:
```bash
# Primjer instalacije u /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Instalacija

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Izvršna datoteka bit će dostupna u `target/release/oracle2vortex`.

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

### Opcije

| Opcija | Kratka | Opis | Zadano |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Putanja do SQL datoteke s upitom | (obavezno) |
| `--output` | `-o` | Putanja izlazne Vortex datoteke | (obavezno) |
| `--host` | | Oracle host | (obavezno) |
| `--port` | | Oracle port | 1521 |
| `--user` | `-u` | Oracle korisnik | (obavezno) |
| `--password` | `-p` | Oracle lozinka | (obavezno) |
| `--sid` | | Oracle SID ili naziv usluge | (obavezno) |
| `--sqlcl-path` | | Putanja do SQLcl izvršne datoteke | `sql` |
| `--auto-batch-rows` | | Broj redaka po grupi (0 = onemogućeno) | 0 |

### Auto-grupna obrada (velike tablice)

Za obradu tablica s milijunima ili milijardama redaka s konstantnom upotrebom memorije, koristite opciju `--auto-batch-rows`:

```bash
# Obrada u grupama od 50000 redaka
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

**Kako funkcionira:**
1. Automatski umota upit s `OFFSET/FETCH`
2. Izvršava SQLcl više puta (jednom po grupi)
3. Akumulira sve rezultate u memoriju
4. Piše jednu Vortex datoteku koja sadrži sve podatke

**Ograničenja:**
- Zahtijeva Oracle 12c+ (OFFSET/FETCH sintaksa)
- Vaš upit NE smije već sadržavati OFFSET/FETCH ili ROWNUM
- Preporučeno: dodajte ORDER BY za dosljedan redoslijed

**Memorija:** S auto-grupnom obradom, korištena memorija = veličina grupe × 2 (JSON + Vortex)  
Primjer: 50000 redaka × 1 KB = 100 MB po grupi (umjesto učitavanja cijele tablice)

**Pogledajte također:** `BATCH_PROCESSING.md` i `README_LARGE_DATASETS.md` za više detalja.

### Primjer s SQL datotekom

Kreirajte datoteku `query.sql`:

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

Zatim izvršite:

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

## Funkcioniranje

1. **Čitanje SQL**: SQL datoteka se učitava u memoriju
2. **Pokretanje SQLcl**: Pokretanje procesa s Oracle vezom
3. **Konfiguracija sesije**:
   - `SET SQLFORMAT JSON` za JSON izvoz
   - `SET NLS_NUMERIC_CHARACTERS='.,';` za izbjegavanje problema s locale
4. **Izvršavanje upita**: SQL upit se šalje putem stdin
5. **Hvatanje izlaza**: Potpuno čitanje JSON stdout
6. **Ekstrakcija JSON**: Izolacija strukture `{"results":[{"items":[...]}]}`
7. **Inferencija sheme**: Vortex shema se automatski izvodi iz prvog zapisa
8. **Konverzija zapisa**: Svaki JSON objekt se transformira u Vortex stupce
9. **Pisanje datoteke**: Binarna Vortex datoteka se kreira s Tokio sesijom

## Podržani tipovi podataka

Konverzija JSON tipova u Vortex tipove odvija se automatski:

| JSON tip | Vortex tip | Nullable | Napomene |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Izveden kao nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (cijeli broj) | `Primitive(I64)` | ✅ | Detektiran s `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Detektiran s `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serijaliziran kao JSON string |
| `object` | `Utf8` | ✅ | Serijaliziran kao JSON string |

**Napomena**: Svi tipovi su nullable za rukovanje Oracle NULL vrijednostima.

## Logovi i debugiranje

Aplikacija koristi `tracing` za logove. Poruke se prikazuju na stderr s razinom loga.

Logovi uključuju:
- Povezivanje na Oracle
- Broj obrađenih zapisa
- Izvedena shema
- Greške i upozorenja

## Provjera generiranih Vortex datoteka

Za provjeru generiranih datoteka, koristite alat `vx`:

```bash
# Instalacija vx (Vortex CLI alat)
cargo install vortex-vx

# Pregledavanje Vortex datoteke
vx browse output.vortex

# Prikazivanje metapodataka
vx info output.vortex
```

## Ograničenja i razmatranja

- **Složeni tipovi**: Ugniježđeni JSON objekti i polja se serijaliziraju u nizove
- **Buffer u memoriji**: Zapisi se trenutno spremaju u buffer prije pisanja (buduća optimizacija moguća)
- **Fiksna shema**: Izvedena samo iz prvog zapisa (sljedeći zapisi moraju odgovarati)
- **Sigurnost**: Lozinka se prosljeđuje kao CLI argument (vidljiva s `ps`). Koristite varijable okoline u produkciji.

## Razvoj

### Build u debug načinu

```bash
cargo build
```

### Build u release načinu

```bash
cargo build --release
```

Binarna datoteka bit će u `target/release/oracle2vortex` (~46 MB u release).

### Testovi

```bash
cargo test
```

### Ručni testovi

Testne datoteke s vjerodajnicama su u `tests_local/` (gitignored):

```bash
# Kreiranje testnih upita
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Izvršavanje
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

Ovaj projekt je licenciran pod EUPL-1.2 (European Union Public Licence v. 1.2).

**VAŽNO - Ograničenje komercijalne uporabe:**  
Komercijalna uporaba ovog softvera zabranjena je bez prethodnog pisanog odobrenja autora.  
Za bilo kakve zahtjeve za komercijalnom licencom, kontaktirajte: **oracle2vortex@amilto.com**

Pogledajte datoteku [LICENSE](LICENSE) za potpuni tekst licence.

## Autor

**William Gacquer**  
Kontakt: oracle2vortex@amilto.com

## Povijest testova

Projekt je validiran na proizvodnoj Oracle bazi podataka:

- ✅ **Jednostavan test**: 10 zapisa, 3 stupca → 5.5 KB
- ✅ **Složen test**: 100 zapisa, 417 stupaca → 1.3 MB
- ✅ **Validacija**: Datoteke čitljive s `vx browse` (Vortex v0.58)

## Struktura projekta

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Ova datoteka
├── IMPLEMENTATION.md       # Tehnička dokumentacija
├── .gitignore             # Isključuje tests_local/ i vjerodajnice
├── src/
│   ├── main.rs            # Entry point s tokio runtime
│   ├── cli.rs             # Obrada Clap argumenata
│   ├── sqlcl.rs           # SQLcl proces s CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Konverzija JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Potpuna orkestracija
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Primjer upita
└── tests_local/           # Testovi s vjerodajnicama (gitignored)
```

## Glavne ovisnosti

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
