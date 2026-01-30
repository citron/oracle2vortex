# oracle2vortex

O aplicație CLI care exportă tabele Oracle în format Vortex prin SQLcl cu streaming JSON.

## Descriere

`oracle2vortex` permite exportul de date Oracle utilizând:
- **SQLcl** pentru conexiune și export nativ JSON
- **Streaming** pentru procesarea datelor în timp real fără a aștepta finalul exportului
- **Conversie automată** în format columnar Vortex cu inferență de schemă

✅ **Proiect finalizat și testat în producție** - Validat cu un tabel de 417 coloane pe o bază de date reală.

## Cerințe preliminare

- **Rust nightly** (necesar pentru crate-urile Vortex)
- **SQLcl** instalat (sau specificați calea cu `--sqlcl-path`)
- O bază de date Oracle accesibilă

### Instalare Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Instalare SQLcl

Descărcați SQLcl de la: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Sau pe Linux:
```bash
# Exemplu pentru instalare în /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Instalare

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Executabilul va fi disponibil în `target/release/oracle2vortex`.

## Utilizare

### Sintaxă de bază

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

### Opțiuni

| Opțiune | Scurtă | Descriere | Implicit |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Calea către fișierul SQL care conține interogarea | (obligatoriu) |
| `--output` | `-o` | Calea fișierului Vortex de ieșire | (obligatoriu) |
| `--host` | | Gazdă Oracle | (obligatoriu) |
| `--port` | | Port Oracle | 1521 |
| `--user` | `-u` | Utilizator Oracle | (obligatoriu) |
| `--password` | `-p` | Parolă Oracle | (obligatoriu) |
| `--sid` | | SID sau nume de serviciu Oracle | (obligatoriu) |
| `--sqlcl-path` | | Calea către executabilul SQLcl | `sql` |
| `--auto-batch-rows` | | Număr de rânduri pe lot (0 = dezactivat) | 0 |
| `--skip-lobs` | | Omite tipurile LOB Oracle (CLOB, BLOB, NCLOB) | false |

### Auto-procesare pe loturi (tabele mari)

Pentru procesarea tabelelor cu milioane sau miliarde de rânduri cu utilizare constantă a memoriei, folosiți opțiunea `--auto-batch-rows`:

```bash
# Procesare în loturi de 50000 rânduri
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

**Cum funcționează:**
1. Înfășoară automat interogarea cu `OFFSET/FETCH`
2. Execută SQLcl de mai multe ori (o dată pe lot)
3. Acumulează toate rezultatele în memorie
4. Scrie un singur fișier Vortex care conține toate datele

**Limitări:**
- Necesită Oracle 12c+ (sintaxă OFFSET/FETCH)
- Interogarea NU trebuie să conțină deja OFFSET/FETCH sau ROWNUM
- Recomandat: adăugați ORDER BY pentru o ordine consistentă

**Memorie:** Cu auto-procesare pe loturi, memorie utilizată = dimensiune lot × 2 (JSON + Vortex)  
Exemplu: 50000 rânduri × 1 KB = 100 MB per lot (în loc de încărcarea întregului tabel)

**Vezi și:** `BATCH_PROCESSING.md` și `README_LARGE_DATASETS.md` pentru mai multe detalii.

### Omiterea coloanelor LOB

Tipurile LOB Oracle (CLOB, BLOB, NCLOB) pot fi foarte mari și pot să nu fie necesare pentru analiză. Folosiți `--skip-lobs` pentru a le exclude:

```bash
# Omite coloanele LOB pentru a reduce dimensiunea fișierului și a îmbunătăți performanța
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

**Cum funcționează:**
- Detectează și filtrează automat coloanele care conțin date LOB
- LOB-urile sunt identificate după dimensiune (> 4000 caractere) sau indicatori binari
- Primul înregistrare jurnalizată va arăta câte coloane au fost omise
- Reduce semnificativ dimensiunea fișierului și utilizarea memoriei pentru tabelele cu câmpuri text/binare mari

**Cazuri de utilizare:**
- Exportarea tabelelor de metadate cu câmpuri de descriere
- Lucrul cu tabele care conțin documente XML sau JSON mari
- Concentrarea pe date structurate ignorând conținutul binar
- Optimizarea performanței pentru tabele cu multe coloane mari

### Exemplu cu fișier SQL

Creați un fișier `query.sql`:

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

Apoi executați:

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

## Arhitectură

```
┌─────────────┐
│  Fișier     │
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
│  Fișier .vortex          │
│  (columnar binary)       │
└──────────────────────────┘
```

## Funcționare

1. **Citire SQL**: Fișierul SQL este încărcat în memorie
2. **Lansare SQLcl**: Pornirea procesului cu conexiune Oracle
3. **Configurare sesiune**:
   - `SET SQLFORMAT JSON` pentru export JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` pentru a evita problemele de locale
4. **Execuție interogare**: Interogarea SQL este trimisă via stdin
5. **Capturare ieșire**: Citire completă a stdout JSON
6. **Extracție JSON**: Izolarea structurii `{"results":[{"items":[...]}]}`
7. **Inferență schemă**: Schema Vortex este dedusă automat din prima înregistrare
8. **Conversie înregistrări**: Fiecare obiect JSON este transformat în coloane Vortex
9. **Scriere fișier**: Fișier binar Vortex creat cu sesiune Tokio

## Tipuri de date suportate

Conversia tipurilor JSON către Vortex se face automat:

| Tip JSON | Tip Vortex | Nullable | Note |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Dedus ca string nullable |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (întreg) | `Primitive(I64)` | ✅ | Detectat cu `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Detectat cu `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serializat ca string JSON |
| `object` | `Utf8` | ✅ | Serializat ca string JSON |

**Notă**: Toate tipurile sunt nullable pentru gestionarea valorilor Oracle NULL.

## Jurnale și depanare

Aplicația utilizează `tracing` pentru jurnale. Mesajele sunt afișate pe stderr cu nivelul de jurnal.

Jurnalele includ:
- Conexiune la Oracle
- Număr de înregistrări procesate
- Schema inferată
- Erori și avertismente

## Verificarea fișierelor Vortex generate

Pentru verificarea fișierelor generate, utilizați instrumentul `vx`:

```bash
# Instalare vx (instrument Vortex CLI)
cargo install vortex-vx

# Explorare fișier Vortex
vx browse output.vortex

# Afișare metadate
vx info output.vortex
```

## Limitări și considerații

- **Tipuri complexe**: Obiectele JSON imbricate și array-urile sunt serializate în șiruri
- **Buffer în memorie**: Înregistrările sunt în prezent bufferizate înainte de scriere (optimizare viitoare posibilă)
- **Schemă fixă**: Inferată doar din prima înregistrare (înregistrările următoare trebuie să corespundă)
- **Securitate**: Parola este trecută ca argument CLI (vizibilă cu `ps`). Utilizați variabile de mediu în producție.
- **Tipuri LOB**: În mod implicit, coloanele LOB (CLOB, BLOB, NCLOB) sunt incluse. Folosiți `--skip-lobs` pentru a le exclude pentru performanță mai bună și fișiere mai mici.

## Dezvoltare

### Build în modul debug

```bash
cargo build
```

### Build în modul release

```bash
cargo build --release
```

Binarul va fi în `target/release/oracle2vortex` (~46 MB în release).

### Teste

```bash
cargo test
```

### Teste manuale

Fișierele de test cu credențiale sunt în `tests_local/` (gitignored):

```bash
# Creați interogări de test
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Executați
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licență

Copyright (c) 2026 William Gacquer

Acest proiect este licențiat sub EUPL-1.2 (European Union Public Licence v. 1.2).

**IMPORTANT - Restricție de utilizare comercială:**  
Utilizarea comercială a acestui software este interzisă fără acordul scris prealabil al autorului.  
Pentru orice solicitare de licență comercială, contactați: **oracle2vortex@amilto.com**

Consultați fișierul [LICENSE](LICENSE) pentru textul complet al licenței.

## Autor

**William Gacquer**  
Contact: oracle2vortex@amilto.com

## Istoric teste

Proiectul a fost validat pe o bază de date Oracle de producție:

- ✅ **Test simplu**: 10 înregistrări, 3 coloane → 5.5 KB
- ✅ **Test complex**: 100 înregistrări, 417 coloane → 1.3 MB
- ✅ **Validare**: Fișiere lizibile cu `vx browse` (Vortex v0.58)

## Structura proiectului

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Acest fișier
├── IMPLEMENTATION.md       # Documentație tehnică
├── .gitignore             # Exclude tests_local/ și credențiale
├── src/
│   ├── main.rs            # Entry point cu runtime tokio
│   ├── cli.rs             # Parsare argumente Clap
│   ├── sqlcl.rs           # Proces SQLcl cu CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Conversie JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Orchestrare completă
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Exemplu de interogare
└── tests_local/           # Teste cu credențiale (gitignored)
```

## Dependențe principale

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Resurse

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
