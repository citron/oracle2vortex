# oracle2vortex

Aplikacja CLI, która ekstraktuje tabele Oracle do formatu Vortex przez SQLcl ze strumieniowaniem JSON.

## Opis

`oracle2vortex` umożliwia eksport danych Oracle wykorzystując:
- **SQLcl** do połączenia i natywnego eksportu JSON
- **Streaming** do przetwarzania danych w locie bez oczekiwania na zakończenie eksportu
- **Automatyczna konwersja** do kolumnowego formatu Vortex z inferencją schematu

✅ **Projekt ukończony i przetestowany w produkcji** - Zwalidowany tabelą o 417 kolumnach w prawdziwej bazie danych.

## Wymagania wstępne

- **Rust nightly** (wymagany przez crate'y Vortex)
- **SQLcl** zainstalowany (lub określ ścieżkę za pomocą `--sqlcl-path`)
- Dostępna baza danych Oracle

### Instalacja Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Instalacja SQLcl

Pobierz SQLcl z: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Lub na Linux:
```bash
# Przykład instalacji w /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Instalacja

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Plik wykonywalny będzie dostępny w `target/release/oracle2vortex`.

## Użycie

### Podstawowa składnia

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

### Opcje

| Opcja | Skrót | Opis | Domyślnie |
|-------|-------|------|-----------|
| `--sql-file` | `-f` | Ścieżka do pliku SQL zawierającego zapytanie | (wymagane) |
| `--output` | `-o` | Ścieżka wyjściowego pliku Vortex | (wymagane) |
| `--host` | | Host Oracle | (wymagane) |
| `--port` | | Port Oracle | 1521 |
| `--user` | `-u` | Użytkownik Oracle | (wymagane) |
| `--password` | `-p` | Hasło Oracle | (wymagane) |
| `--sid` | | SID lub nazwa usługi Oracle | (wymagane) |
| `--sqlcl-path` | | Ścieżka do pliku wykonywalnego SQLcl | `sql` |
| `--auto-batch-rows` | | Liczba wierszy na partię (0 = wyłączone) | 0 |
| `--skip-lobs` | | Pomiń typy LOB Oracle (CLOB, BLOB, NCLOB) | false |

### Auto-Batching (Duże tabele)

Aby przetwarzać tabele z milionami lub miliardami wierszy przy stałym użyciu pamięci, użyj opcji `--auto-batch-rows`:

```bash
# Przetwarzaj w partiach po 50000 wierszy
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

**Jak to działa:**
1. Automatycznie opakowuje zapytanie w `OFFSET/FETCH`
2. Wykonuje SQLcl wielokrotnie (raz na partię)
3. Akumuluje wszystkie wyniki w pamięci
4. Zapisuje pojedynczy plik Vortex zawierający wszystkie dane

**Ograniczenia:**
- Wymaga Oracle 12c+ (składnia OFFSET/FETCH)
- Zapytanie NIE może już zawierać OFFSET/FETCH lub ROWNUM
- Zalecane: dodaj ORDER BY dla spójnej kolejności

**Pamięć:** Z auto-batching, użyta pamięć = rozmiar partii × 2 (JSON + Vortex)  
Przykład: 50000 wierszy × 1 KB = 100 MB na partię (zamiast ładowania całej tabeli)

**Zobacz także:** `BATCH_PROCESSING.md` i `README_LARGE_DATASETS.md` po więcej szczegółów.

### Pomijanie kolumn LOB

Typy LOB Oracle (CLOB, BLOB, NCLOB) mogą być bardzo duże i mogą nie być potrzebne do analizy. Użyj `--skip-lobs`, aby je wykluczyć:

```bash
# Pomiń kolumny LOB, aby zmniejszyć rozmiar pliku i poprawić wydajność
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

**Jak to działa:**
- Automatycznie wykrywa i filtruje kolumny zawierające dane LOB
- LOB są identyfikowane po rozmiarze (> 4000 znaków) lub wskaźnikach binarnych
- Pierwszy zarejestrowany rekord pokaże, ile kolumn zostało pominiętych
- Znacząco zmniejsza rozmiar pliku i użycie pamięci dla tabel z dużymi polami tekstowymi/binarnymi

**Przypadki użycia:**
- Eksportowanie tabel metadanych z polami opisu
- Praca z tabelami zawierającymi dokumenty XML lub duże dokumenty JSON
- Skupienie się na danych strukturalnych z pominięciem zawartości binarnej
- Optymalizacja wydajności dla tabel z wieloma dużymi kolumnami

### Przykład z plikiem SQL

Utwórz plik `query.sql`:

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

Następnie wykonaj:

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

## Architektura

```
┌─────────────┐
│  Plik SQL   │
│             │
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
│  Plik .vortex            │
│  (columnar binary)       │
└──────────────────────────┘
```

## Działanie

1. **Odczyt SQL**: Plik SQL jest ładowany do pamięci
2. **Uruchomienie SQLcl**: Proces rozpoczyna się z połączeniem Oracle
3. **Konfiguracja sesji**:
   - `SET SQLFORMAT JSON` do eksportu JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` aby uniknąć problemów z locale
4. **Wykonanie zapytania**: Zapytanie SQL jest wysyłane przez stdin
5. **Przechwytywanie wyjścia**: Pełny odczyt stdout JSON
6. **Ekstrakcja JSON**: Izolacja struktury `{"results":[{"items":[...]}]}`
7. **Inferencja schematu**: Schemat Vortex jest automatycznie wywnioskowany z pierwszego rekordu
8. **Konwersja rekordów**: Każdy obiekt JSON jest przekształcany w kolumny Vortex
9. **Zapis pliku**: Binarny plik Vortex tworzony z sesją Tokio

## Obsługiwane typy danych

Konwersja typów JSON do Vortex jest automatyczna:

| Typ JSON | Typ Vortex | Nullable | Uwagi |
|----------|------------|----------|-------|
| `null` | `Utf8` | ✅ | Wywnioskowany jako nullable string |
| `boolean` | `Bool` | ✅ | Przez BoolArray |
| `number` (integer) | `Primitive(I64)` | ✅ | Wykrywany z `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Wykrywany z `is_f64() == true` |
| `string` | `Utf8` | ✅ | Przez VarBinArray |
| `array` | `Utf8` | ✅ | Serializowany jako string JSON |
| `object` | `Utf8` | ✅ | Serializowany jako string JSON |

**Uwaga**: Wszystkie typy są nullable do obsługi wartości Oracle NULL.

## Logowanie i debugowanie

Aplikacja używa `tracing` do logów. Komunikaty są wyświetlane na stderr z poziomem logu.

Logi zawierają:
- Połączenie z Oracle
- Liczba przetworzonych rekordów
- Wywnioskowany schemat
- Błędy i ostrzeżenia

## Weryfikacja wygenerowanych plików Vortex

Aby zweryfikować wygenerowane pliki, użyj narzędzia `vx`:

```bash
# Instalacja vx (narzędzie CLI Vortex)
cargo install vortex-vx

# Przeglądaj plik Vortex
vx browse output.vortex

# Wyświetl metadane
vx info output.vortex
```

## Ograniczenia i uwagi

- **Typy złożone**: Zagnieżdżone obiekty JSON i tablice są serializowane do stringów
- **Bufor w pamięci**: Rekordy są obecnie buforowane przed zapisem (możliwa optymalizacja w przyszłości)
- **Stały schemat**: Wywnioskowany tylko z pierwszego rekordu (kolejne rekordy muszą pasować)
- **Bezpieczeństwo**: Hasło jest przekazywane jako argument CLI (widoczne przez `ps`). Użyj zmiennych środowiskowych w produkcji.
- **Typy LOB**: Domyślnie kolumny LOB (CLOB, BLOB, NCLOB) są uwzględniane. Użyj `--skip-lobs`, aby je wykluczyć dla lepszej wydajności i mniejszych rozmiarów plików.

## Rozwój

### Build w trybie debug

```bash
cargo build
```

### Build w trybie release

```bash
cargo build --release
```

Plik binarny będzie w `target/release/oracle2vortex` (~46 MB w release).

### Testy

```bash
cargo test
```

### Testy manualne

Pliki testowe z danymi uwierzytelniającymi są w `tests_local/` (gitignored):

```bash
# Utwórz zapytania testowe
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Wykonaj
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licencja

Copyright (c) 2026 William Gacquer

Ten projekt jest licencjonowany na podstawie EUPL-1.2 (European Union Public Licence v. 1.2).

**WAŻNE - Ograniczenie użytku komercyjnego:**  
Komercyjne wykorzystanie tego oprogramowania jest zabronione bez uprzedniej pisemnej zgody autora.  
W sprawie wszelkich próśb o licencję komercyjną prosimy o kontakt: **oracle2vortex@amilto.com**

Zobacz plik [LICENSE](LICENSE) dla pełnego tekstu licencji.

## Autor

**William Gacquer**  
Kontakt: oracle2vortex@amilto.com

## Historia testów

Projekt został zwalidowany na produkcyjnej bazie danych Oracle:

- ✅ **Test prosty**: 10 rekordów, 3 kolumny → 5,5 KB
- ✅ **Test złożony**: 100 rekordów, 417 kolumn → 1,3 MB
- ✅ **Walidacja**: Pliki czytelne za pomocą `vx browse` (Vortex v0.58)

## Struktura projektu

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Ten plik
├── IMPLEMENTATION.md       # Dokumentacja techniczna
├── .gitignore             # Wyklucza tests_local/ i dane uwierzytelniające
├── src/
│   ├── main.rs            # Entry point z tokio runtime
│   ├── cli.rs             # Parsowanie argumentów Clap
│   ├── sqlcl.rs           # Proces SQLcl z CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Konwersja JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Pełna orkiestracja
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Przykładowe zapytanie
└── tests_local/           # Testy z danymi uwierzytelniającymi (gitignored)
```

## Główne zależności

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Zasoby

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
