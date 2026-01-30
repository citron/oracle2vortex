# oracle2vortex

CLI programa Oracle lentelių eksportavimui į Vortex formatą per SQLcl su JSON srautiniu perdavimu.

## Aprašymas

`oracle2vortex` leidžia eksportuoti Oracle duomenis naudojant:
- **SQLcl** ryšiui ir natyviam JSON eksportui
- **Srautinį perdavimą** duomenų apdorojimui skrydžio metu nelaukiant eksporto pabaigos
- **Automatinį konvertavimą** į stulpelinį Vortex formatą su schemos išvedimu

✅ **Projektas baigtas ir išbandytas gamyboje** - Patvirtintas su 417 stulpelių lentele tikroje duomenų bazėje.

## Būtinos sąlygos

- **Rust nightly** (reikalauja Vortex crate-ai)
- **SQLcl** įdiegtas (arba nurodykite kelią su `--sqlcl-path`)
- Prieinama Oracle duomenų bazė

### Rust nightly diegimas

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### SQLcl diegimas

Atsisiųskite SQLcl iš: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Arba Linux:
```bash
# Pavyzdys diegimui į /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Diegimas

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Vykdomasis failas bus prieinamas `target/release/oracle2vortex`.

## Naudojimas

### Pagrindinė sintaksė

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

### Parinktys

| Parinktis | Trumpa | Aprašymas | Numatytasis |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Kelias į SQL failą su užklausa | (privalomas) |
| `--output` | `-o` | Išvesties Vortex failo kelias | (privalomas) |
| `--host` | | Oracle pagrindinis kompiuteris | (privalomas) |
| `--port` | | Oracle prievadas | 1521 |
| `--user` | `-u` | Oracle vartotojas | (privalomas) |
| `--password` | `-p` | Oracle slaptažodis | (privalomas) |
| `--sid` | | Oracle SID arba paslaugos pavadinimas | (privalomas) |
| `--sqlcl-path` | | Kelias į SQLcl vykdomąjį failą | `sql` |
| `--auto-batch-rows` | | Eilučių skaičius partijoje (0 = išjungta) | 0 |
| `--skip-lobs` | | Praleisti Oracle LOB tipus (CLOB, BLOB, NCLOB) | false |

### Automatinis partijinis apdorojimas (didelės lentelės)

Lentelių su milijonais ar milijardais eilučių apdorojimui su pastovia atminties naudojimu naudokite parinktį `--auto-batch-rows`:

```bash
# Apdorojimas 50000 eilučių partijomis
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

**Kaip tai veikia:**
1. Automatiškai įvynioja jūsų užklausą su `OFFSET/FETCH`
2. Vykdo SQLcl kelis kartus (vieną kartą partijai)
3. Kaupia visus rezultatus atmintyje
4. Rašo vieną Vortex failą su visais duomenimis

**Apribojimai:**
- Reikalauja Oracle 12c+ (OFFSET/FETCH sintaksė)
- Jūsų užklausa NETURI jau turėti OFFSET/FETCH arba ROWNUM
- Rekomenduojama: pridėkite ORDER BY nuosekliai tvarkai

**Atmintis:** Su automatiniu partijiniu apdorojimu, naudojama atmintis = partijos dydis × 2 (JSON + Vortex)  
Pavyzdys: 50000 eilučių × 1 KB = 100 MB partijai (vietoj visos lentelės įkėlimo)

**Taip pat žiūrėkite:** `BATCH_PROCESSING.md` ir `README_LARGE_DATASETS.md` daugiau detalių.

### LOB stulpelių praleidimas

Oracle LOB tipai (CLOB, BLOB, NCLOB) gali būti labai dideli ir gali nebūti reikalingi analizei. Naudokite `--skip-lobs`, kad juos išskirtumėte:

```bash
# Praleisti LOB stulpelius, kad sumažintumėte failo dydį ir pagerinti našumą
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

**Kaip tai veikia:**
- Automatiškai aptinka ir filtruoja stulpelius, kuriuose yra LOB duomenų
- LOB identifikuojami pagal dydį (> 4000 simbolių) arba dvejetainius indikatorius
- Pirmas užregistruotas įrašas parodys, kiek stulpelių buvo praleista
- Žymiai sumažina failo dydį ir atminties naudojimą lentelėms su dideliais teksto/dvejetainiais laukais

**Naudojimo atvejai:**
- Metaduomenų lentelių eksportavimas su aprašymo laukais
- Darbas su lentelėmis, kuriose yra XML ar dideli JSON dokumentai
- Dėmesio sutelkimas į struktūrinius duomenis ignoruojant dvejetainį turinį
- Našumo optimizavimas lentelėms su daug didelių stulpelių

### Pavyzdys su SQL failu

Sukurkite failą `query.sql`:

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

Tada vykdykite:

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

## Architektūra

```
┌─────────────┐
│  SQL        │
│  failas     │
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
│  .vortex failas          │
│  (columnar binary)       │
└──────────────────────────┘
```

## Veikimas

1. **SQL skaitymas**: SQL failas įkeliamas į atmintį
2. **SQLcl paleidimas**: Proceso paleidimas su Oracle ryšiu
3. **Sesijos konfigūravimas**:
   - `SET SQLFORMAT JSON` JSON eksportui
   - `SET NLS_NUMERIC_CHARACTERS='.,';` locale problemų išvengimui
4. **Užklausos vykdymas**: SQL užklausa siunčiama per stdin
5. **Išvesties fiksavimas**: Visas JSON stdout skaitymas
6. **JSON išskyrimas**: Struktūros `{"results":[{"items":[...]}]}` izoliavimas
7. **Schemos išvedimas**: Vortex schema automatiškai išvedama iš pirmo įrašo
8. **Įrašų konvertavimas**: Kiekvienas JSON objektas transformuojamas į Vortex stulpelius
9. **Failo rašymas**: Dvejetainis Vortex failas sukuriamas su Tokio sesija

## Palaikomi duomenų tipai

JSON tipų konvertavimas į Vortex tipus vyksta automatiškai:

| JSON tipas | Vortex tipas | Nullable | Pastabos |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Išvestas kaip nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (sveikasis) | `Primitive(I64)` | ✅ | Aptiktas su `is_f64() == false` |
| `number` (slankusis) | `Primitive(F64)` | ✅ | Aptiktas su `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serializuotas kaip JSON eilutė |
| `object` | `Utf8` | ✅ | Serializuotas kaip JSON eilutė |

**Pastaba**: Visi tipai yra nullable Oracle NULL reikšmių tvarkymui.

## Žurnalai ir derinimas

Programa naudoja `tracing` žurnalams. Pranešimai rodomi stderr su žurnalo lygiu.

Žurnalai apima:
- Oracle ryšys
- Apdorotų įrašų skaičius
- Išvesta schema
- Klaidos ir įspėjimai

## Sugeneruotų Vortex failų patikrinimas

Sugeneruotų failų patikrinimui naudokite `vx` įrankį:

```bash
# vx diegimas (Vortex CLI įrankis)
cargo install vortex-vx

# Vortex failo naršymas
vx browse output.vortex

# Metaduomenų rodymas
vx info output.vortex
```

## Apribojimai ir svarstymai

- **Sudėtingi tipai**: Įdėti JSON objektai ir masyvai serializuojami į eilutes
- **Buferis atmintyje**: Įrašai šiuo metu buferizuojami prieš rašymą (galima būsima optimizacija)
- **Fiksuota schema**: Išvesta tik iš pirmo įrašo (tolesni įrašai turi atitikti)
- **Saugumas**: Slaptažodis perduodamas kaip CLI argumentas (matomas su `ps`). Gamyboje naudokite aplinkos kintamuosius.
- **LOB tipai**: Pagal numatytuosius nustatymus LOB stulpeliai (CLOB, BLOB, NCLOB) įtraukti. Naudokite `--skip-lobs`, kad juos išskirtumėte geresniam našumui ir mažesniems failų dydžiams.

## Kūrimas

### Build debug režimu

```bash
cargo build
```

### Build release režimu

```bash
cargo build --release
```

Dvejetainis failas bus `target/release/oracle2vortex` (~46 MB release režimu).

### Testai

```bash
cargo test
```

### Rankiniai testai

Testiniai failai su kredencialais yra `tests_local/` (gitignored):

```bash
# Testinių užklausų kūrimas
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Vykdymas
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licencija

Copyright (c) 2026 William Gacquer

Šis projektas licencijuotas pagal EUPL-1.2 (European Union Public Licence v. 1.2).

**SVARBU - Komercinės naudojimo apribojimas:**  
Šios programinės įrangos komercinis naudojimas draudžiamas be išankstinio autoriaus raštiško sutikimo.  
Komercinės licencijos užklausoms kreipkitės: **oracle2vortex@amilto.com**

Žiūrėkite [LICENSE](LICENSE) failą pilnam licencijos tekstui.

## Autorius

**William Gacquer**  
Kontaktai: oracle2vortex@amilto.com

## Testų istorija

Projektas patvirtintas Oracle gamybos duomenų bazėje:

- ✅ **Paprastas testas**: 10 įrašų, 3 stulpeliai → 5.5 KB
- ✅ **Sudėtingas testas**: 100 įrašų, 417 stulpelių → 1.3 MB
- ✅ **Patvirtinimas**: Failai skaitomi su `vx browse` (Vortex v0.58)

## Projekto struktūra

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Šis failas
├── IMPLEMENTATION.md       # Techninė dokumentacija
├── .gitignore             # Neįtraukia tests_local/ ir kredencialų
├── src/
│   ├── main.rs            # Entry point su tokio runtime
│   ├── cli.rs             # Clap argumentų apdorojimas
│   ├── sqlcl.rs           # SQLcl procesas su CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Konvertavimas JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Pilna orkestracija
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Pavyzdinė užklausa
└── tests_local/           # Testai su kredencialais (gitignored)
```

## Pagrindinės priklausomybės

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Ištekliai

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
