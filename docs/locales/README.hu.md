# oracle2vortex

CLI alkalmazás Oracle táblák exportálásához Vortex formátumba SQLcl-en keresztül JSON streaming-gel.

## Leírás

Az `oracle2vortex` lehetővé teszi Oracle adatok exportálását a következők használatával:
- **SQLcl** a kapcsolathoz és natív JSON exporthoz
- **Streaming** az adatok menet közbeni feldolgozásához, anélkül hogy megvárná az export befejezését
- **Automatikus konverzió** oszlopos Vortex formátumba séma következtetéssel

✅ **Projekt befejezve és éles környezetben tesztelve** - Érvényesítve 417 oszlopos táblával valós adatbázison.

## Előfeltételek

- **Rust nightly** (Vortex crate-ek igénylik)
- **SQLcl** telepítve (vagy adja meg az útvonalat `--sqlcl-path` opcióval)
- Elérhető Oracle adatbázis

### Rust nightly telepítés

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### SQLcl telepítés

Töltse le a SQLcl-t innen: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Vagy Linuxon:
```bash
# Példa telepítésre /opt/oracle/sqlcl/ könyvtárba
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Telepítés

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

A végrehajtható fájl elérhető lesz a `target/release/oracle2vortex` útvonalon.

## Használat

### Alapszintaxis

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

### Opciók

| Opció | Rövid | Leírás | Alapértelmezett |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | A lekérdezést tartalmazó SQL fájl útvonala | (kötelező) |
| `--output` | `-o` | A kimeneti Vortex fájl útvonala | (kötelező) |
| `--host` | | Oracle hoszt | (kötelező) |
| `--port` | | Oracle port | 1521 |
| `--user` | `-u` | Oracle felhasználó | (kötelező) |
| `--password` | `-p` | Oracle jelszó | (kötelező) |
| `--sid` | | Oracle SID vagy szolgáltatásnév | (kötelező) |
| `--sqlcl-path` | | SQLcl végrehajtható útvonala | `sql` |
| `--auto-batch-rows` | | Sorok száma kötegenkénként (0 = kikapcsolva) | 0 |

### Auto-kötegelt feldolgozás (nagy táblák)

Millió vagy milliárd soros táblák feldolgozásához állandó memóriahasználattal, használja az `--auto-batch-rows` opciót:

```bash
# Feldolgozás 50000 soros kötegekben
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

**Működés:**
1. Automatikusan becsomagolja a lekérdezést `OFFSET/FETCH` utasítással
2. Többször futtatja a SQLcl-t (egyszer kötegenkénként)
3. Összegyűjti az összes eredményt a memóriában
4. Egyetlen Vortex fájlt ír, amely tartalmazza az összes adatot

**Korlátozások:**
- Oracle 12c+ szükséges (OFFSET/FETCH szintaxis)
- A lekérdezés NEM tartalmazhat már OFFSET/FETCH vagy ROWNUM utasítást
- Ajánlott: ORDER BY hozzáadása konzisztens sorrend érdekében

**Memória:** Auto-kötegelt feldolgozással, használt memória = köteg mérete × 2 (JSON + Vortex)  
Példa: 50000 sor × 1 KB = 100 MB kötegenként (teljes tábla betöltése helyett)

**Lásd még:** `BATCH_PROCESSING.md` és `README_LARGE_DATASETS.md` további részletekért.

### Példa SQL fájllal

Hozzon létre egy `query.sql` fájlt:

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

Ezután futtassa:

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

## Architektúra

```
┌─────────────┐
│  SQL        │
│  fájl       │
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
│  .vortex fájl            │
│  (columnar binary)       │
└──────────────────────────┘
```

## Működés

1. **SQL olvasás**: Az SQL fájl betöltődik a memóriába
2. **SQLcl indítás**: Folyamat indítása Oracle kapcsolattal
3. **Munkamenet konfiguráció**:
   - `SET SQLFORMAT JSON` a JSON exporthoz
   - `SET NLS_NUMERIC_CHARACTERS='.,';` locale problémák elkerülésére
4. **Lekérdezés végrehajtása**: Az SQL lekérdezés elküldése stdin-en keresztül
5. **Kimenet rögzítése**: A teljes JSON stdout beolvasása
6. **JSON kinyerés**: A `{"results":[{"items":[...]}]}` struktúra elkülönítése
7. **Séma következtetés**: A Vortex séma automatikusan származtatódik az első rekordból
8. **Rekordok konvertálása**: Minden JSON objektum Vortex oszlopokká alakul
9. **Fájl írás**: Bináris Vortex fájl létrehozása Tokio session-nel

## Támogatott adattípusok

A JSON típusok Vortex típusokká alakítása automatikusan történik:

| JSON típus | Vortex típus | Nullable | Megjegyzések |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Nullable string-ként következtetett |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (egész) | `Primitive(I64)` | ✅ | Felismert ha `is_f64() == false` |
| `number` (lebegőpontos) | `Primitive(F64)` | ✅ | Felismert ha `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | JSON string-ként szerializált |
| `object` | `Utf8` | ✅ | JSON string-ként szerializált |

**Megjegyzés**: Minden típus nullable az Oracle NULL értékek kezeléséhez.

## Naplók és hibakeresés

Az alkalmazás `tracing` könyvtárat használ naplózáshoz. Az üzenetek stderr-re kerülnek a naplószinttel.

A naplók tartalmazzák:
- Oracle kapcsolat
- Feldolgozott rekordok száma
- Következtetett séma
- Hibák és figyelmeztetések

## Generált Vortex fájlok ellenőrzése

A generált fájlok ellenőrzéséhez használja a `vx` eszközt:

```bash
# vx telepítése (Vortex CLI eszköz)
cargo install vortex-vx

# Vortex fájl böngészése
vx browse output.vortex

# Metaadatok megjelenítése
vx info output.vortex
```

## Korlátozások és megfontolások

- **Összetett típusok**: A beágyazott JSON objektumok és tömbök string-ekké szerializálódnak
- **Memória puffer**: A rekordok jelenleg pufferelődnek írás előtt (jövőbeli optimalizálás lehetséges)
- **Fix séma**: Csak az első rekordból következtetett (a további rekordoknak meg kell egyezniük)
- **Biztonság**: A jelszó CLI argumentumként kerül átadásra (`ps`-sel látható). Használjon környezeti változókat éles környezetben.

## Fejlesztés

### Build debug módban

```bash
cargo build
```

### Build release módban

```bash
cargo build --release
```

A bináris a `target/release/oracle2vortex` útvonalon lesz (~46 MB release módban).

### Tesztek

```bash
cargo test
```

### Manuális tesztek

A hitelesítési adatokat tartalmazó tesztfájlok a `tests_local/` könyvtárban vannak (gitignored):

```bash
# Teszt lekérdezések létrehozása
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Futtatás
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licenc

Copyright (c) 2026 William Gacquer

Ez a projekt EUPL-1.2 (European Union Public Licence v. 1.2) licenc alatt áll.

**FONTOS - Kereskedelmi használat korlátozása:**  
A szoftver kereskedelmi használata tilos a szerző előzetes írásbeli hozzájárulása nélkül.  
Kereskedelmi licenc iránti kérelmekhez vegye fel a kapcsolatot: **oracle2vortex@amilto.com**

Lásd a [LICENSE](LICENSE) fájlt a licenc teljes szövegéért.

## Szerző

**William Gacquer**  
Kapcsolat: oracle2vortex@amilto.com

## Teszt előzmények

A projekt Oracle éles adatbázison lett validálva:

- ✅ **Egyszerű teszt**: 10 rekord, 3 oszlop → 5.5 KB
- ✅ **Összetett teszt**: 100 rekord, 417 oszlop → 1.3 MB
- ✅ **Validálás**: Fájlok olvashatók `vx browse` eszközzel (Vortex v0.58)

## Projekt struktúra

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Ez a fájl
├── IMPLEMENTATION.md       # Technikai dokumentáció
├── .gitignore             # Kizárja a tests_local/ és hitelesítési adatokat
├── src/
│   ├── main.rs            # Entry point tokio runtime-mal
│   ├── cli.rs             # Clap argumentum feldolgozás
│   ├── sqlcl.rs           # SQLcl folyamat CONNECT-tel
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Konverzió JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Teljes orkesztrálás
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Példa lekérdezés
└── tests_local/           # Tesztek hitelesítési adatokkal (gitignored)
```

## Fő függőségek

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Források

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
