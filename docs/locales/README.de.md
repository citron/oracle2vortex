# oracle2vortex

Eine CLI-Anwendung, die Oracle-Tabellen über SQLcl mit JSON-Streaming in das Vortex-Format extrahiert.

## Beschreibung

`oracle2vortex` ermöglicht den Export von Oracle-Daten mit:
- **SQLcl** für Verbindung und nativen JSON-Export
- **Streaming** zur On-the-fly-Verarbeitung von Daten ohne auf Abschluss des Exports zu warten
- **Automatische Konvertierung** in das spaltenbasierte Vortex-Format mit Schema-Inferenz

✅ **Projekt abgeschlossen und produktiv getestet** - Validiert mit einer Tabelle mit 417 Spalten auf einer echten Datenbank.

## Voraussetzungen

- **Rust nightly** (erforderlich für Vortex-Crates)
- **SQLcl** installiert (oder Pfad mit `--sqlcl-path` angeben)
- Eine zugängliche Oracle-Datenbank

### Installation von Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Installation von SQLcl

SQLcl herunterladen von: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Oder unter Linux:
```bash
# Beispiel für Installation in /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Installation

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Die ausführbare Datei wird in `target/release/oracle2vortex` verfügbar sein.

## Verwendung

### Grundlegende Syntax

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

### Optionen

| Option | Kurz | Beschreibung | Standard |
|--------|------|--------------|----------|
| `--sql-file` | `-f` | Pfad zur SQL-Datei mit der Abfrage | (erforderlich) |
| `--output` | `-o` | Pfad der Ausgabe-Vortex-Datei | (erforderlich) |
| `--host` | | Oracle-Host | (erforderlich) |
| `--port` | | Oracle-Port | 1521 |
| `--user` | `-u` | Oracle-Benutzer | (erforderlich) |
| `--password` | `-p` | Oracle-Passwort | (erforderlich) |
| `--sid` | | Oracle-SID oder Dienstname | (erforderlich) |
| `--sqlcl-path` | | Pfad zur SQLcl-Ausführungsdatei | `sql` |
| `--auto-batch-rows` | | Anzahl Zeilen pro Stapel (0 = deaktiviert) | 0 |
| `--skip-lobs` | | Oracle LOB-Typen überspringen (CLOB, BLOB, NCLOB) | false |

### Auto-Batching (Große Tabellen)

Zur Verarbeitung von Tabellen mit Millionen oder Milliarden von Zeilen bei konstantem Speicherverbrauch verwenden Sie die Option `--auto-batch-rows`:

```bash
# Verarbeitung in Stapeln von 50000 Zeilen
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

**Funktionsweise:**
1. Umschließt Ihre Abfrage automatisch mit `OFFSET/FETCH`
2. Führt SQLcl mehrmals aus (einmal pro Stapel)
3. Akkumuliert alle Ergebnisse im Speicher
4. Schreibt eine einzelne Vortex-Datei mit allen Daten

**Einschränkungen:**
- Erfordert Oracle 12c+ (OFFSET/FETCH-Syntax)
- Ihre Abfrage darf NICHT bereits OFFSET/FETCH oder ROWNUM enthalten
- Empfohlen: ORDER BY für konsistente Reihenfolge hinzufügen

**Speicher:** Mit Auto-Batching, genutzter Speicher = Stapelgröße × 2 (JSON + Vortex)  
Beispiel: 50000 Zeilen × 1 KB = 100 MB pro Stapel (anstatt die gesamte Tabelle zu laden)

**Siehe auch:** `BATCH_PROCESSING.md` und `README_LARGE_DATASETS.md` für weitere Details.

### LOB-Spalten überspringen

Oracle LOB-Typen (CLOB, BLOB, NCLOB) können sehr groß sein und werden möglicherweise nicht für die Analyse benötigt. Verwenden Sie `--skip-lobs`, um sie auszuschließen:

```bash
# LOB-Spalten überspringen, um Dateigröße zu reduzieren und Leistung zu verbessern
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

**Funktionsweise:**
- Erkennt und filtert automatisch Spalten mit LOB-Daten
- LOBs werden anhand der Größe (> 4000 Zeichen) oder binärer Indikatoren identifiziert
- Der erste protokollierte Datensatz zeigt, wie viele Spalten übersprungen wurden
- Reduziert erheblich Dateigröße und Speicherverbrauch bei Tabellen mit großen Text-/Binärfeldern

**Anwendungsfälle:**
- Export von Metadatentabellen mit Beschreibungsfeldern
- Arbeiten mit Tabellen, die XML- oder große JSON-Dokumente enthalten
- Konzentration auf strukturierte Daten unter Ignorierung von Binärinhalten
- Leistungsoptimierung für Tabellen mit vielen großen Spalten

### Beispiel mit SQL-Datei

Erstellen Sie eine Datei `query.sql`:

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

Dann ausführen:

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

## Architektur

```
┌─────────────┐
│  SQL-Datei  │
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
│  .vortex-Datei           │
│  (columnar binary)       │
└──────────────────────────┘
```

## Funktionsweise

1. **SQL-Lesen**: Die SQL-Datei wird in den Speicher geladen
2. **SQLcl-Start**: Prozess startet mit Oracle-Verbindung
3. **Sitzungskonfiguration**:
   - `SET SQLFORMAT JSON` für JSON-Export
   - `SET NLS_NUMERIC_CHARACTERS='.,';` zur Vermeidung von Locale-Problemen
4. **Abfrageausführung**: Die SQL-Abfrage wird über stdin gesendet
5. **Ausgabeerfassung**: Vollständiges Lesen von JSON-stdout
6. **JSON-Extraktion**: Isolierung der `{"results":[{"items":[...]}]}`-Struktur
7. **Schema-Inferenz**: Das Vortex-Schema wird automatisch aus dem ersten Datensatz abgeleitet
8. **Datensatzkonvertierung**: Jedes JSON-Objekt wird in Vortex-Spalten umgewandelt
9. **Dateischreiben**: Binäre Vortex-Datei wird mit Tokio-Sitzung erstellt

## Unterstützte Datentypen

Die Konvertierung von JSON- zu Vortex-Typen erfolgt automatisch:

| JSON-Typ | Vortex-Typ | Nullable | Hinweise |
|----------|------------|----------|----------|
| `null` | `Utf8` | ✅ | Als nullable String inferiert |
| `boolean` | `Bool` | ✅ | Über BoolArray |
| `number` (Ganzzahl) | `Primitive(I64)` | ✅ | Erkannt mit `is_f64() == false` |
| `number` (Fließkomma) | `Primitive(F64)` | ✅ | Erkannt mit `is_f64() == true` |
| `string` | `Utf8` | ✅ | Über VarBinArray |
| `array` | `Utf8` | ✅ | Als JSON-String serialisiert |
| `object` | `Utf8` | ✅ | Als JSON-String serialisiert |

**Hinweis**: Alle Typen sind nullable, um Oracle-NULL-Werte zu handhaben.

## Protokollierung und Debugging

Die Anwendung verwendet `tracing` für Protokolle. Nachrichten werden auf stderr mit Log-Level angezeigt.

Protokolle enthalten:
- Oracle-Verbindung
- Anzahl verarbeiteter Datensätze
- Inferiertes Schema
- Fehler und Warnungen

## Verifizierung generierter Vortex-Dateien

Zur Verifizierung generierter Dateien verwenden Sie das `vx`-Tool:

```bash
# Installation von vx (Vortex CLI-Tool)
cargo install vortex-vx

# Vortex-Datei durchsuchen
vx browse output.vortex

# Metadaten anzeigen
vx info output.vortex
```

## Einschränkungen und Überlegungen

- **Komplexe Typen**: Verschachtelte JSON-Objekte und Arrays werden zu Strings serialisiert
- **In-Memory-Puffer**: Datensätze werden derzeit vor dem Schreiben gepuffert (zukünftige Optimierung möglich)
- **Fixes Schema**: Nur aus dem ersten Datensatz inferiert (nachfolgende Datensätze müssen übereinstimmen)
- **Sicherheit**: Passwort wird als CLI-Argument übergeben (sichtbar mit `ps`). In Produktion Umgebungsvariablen verwenden.
- **LOB-Typen**: Standardmäßig werden LOB-Spalten (CLOB, BLOB, NCLOB) einbezogen. Verwenden Sie `--skip-lobs`, um sie für bessere Leistung und kleinere Dateigrößen auszuschließen.

## Entwicklung

### Debug-Build

```bash
cargo build
```

### Release-Build

```bash
cargo build --release
```

Die Binärdatei befindet sich in `target/release/oracle2vortex` (~46 MB im Release).

### Tests

```bash
cargo test
```

### Manuelle Tests

Testdateien mit Anmeldedaten befinden sich in `tests_local/` (gitignored):

```bash
# Testabfragen erstellen
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Ausführen
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Lizenz

Copyright (c) 2026 William Gacquer

Dieses Projekt ist unter EUPL-1.2 (European Union Public Licence v. 1.2) lizenziert.

**WICHTIG - Einschränkung der kommerziellen Nutzung:**  
Die kommerzielle Nutzung dieser Software ist ohne vorherige schriftliche Zustimmung des Autors untersagt.  
Für Anfragen zu einer kommerziellen Lizenz wenden Sie sich bitte an: **oracle2vortex@amilto.com**

Siehe [LICENSE](LICENSE)-Datei für den vollständigen Lizenztext.

## Autor

**William Gacquer**  
Kontakt: oracle2vortex@amilto.com

## Test-Historie

Das Projekt wurde auf einer Oracle-Produktionsdatenbank validiert:

- ✅ **Einfacher Test**: 10 Datensätze, 3 Spalten → 5,5 KB
- ✅ **Komplexer Test**: 100 Datensätze, 417 Spalten → 1,3 MB
- ✅ **Validierung**: Dateien lesbar mit `vx browse` (Vortex v0.58)

## Projektstruktur

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Diese Datei
├── IMPLEMENTATION.md       # Technische Dokumentation
├── .gitignore             # Schließt tests_local/ und Anmeldedaten aus
├── src/
│   ├── main.rs            # Einstiegspunkt mit tokio runtime
│   ├── cli.rs             # Clap-Argument-Parsing
│   ├── sqlcl.rs           # SQLcl-Prozess mit CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # JSON→Vortex-Konvertierung (API 0.58)
│   └── pipeline.rs        # Vollständige Orchestrierung
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Beispielabfrage
└── tests_local/           # Tests mit Anmeldedaten (gitignored)
```

## Hauptabhängigkeiten

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Ressourcen

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
