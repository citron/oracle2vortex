# oracle2vortex

Un'applicazione CLI che estrae tabelle Oracle nel formato Vortex tramite SQLcl con streaming JSON.

## Descrizione

`oracle2vortex` consente di esportare dati Oracle utilizzando:
- **SQLcl** per la connessione e l'esportazione nativa in JSON
- **Streaming** per elaborare i dati al volo senza attendere il completamento dell'esportazione
- **Conversione automatica** nel formato colonnare Vortex con inferenza dello schema

✅ **Progetto completato e testato in produzione** - Validato con una tabella di 417 colonne su un database reale.

## Prerequisiti

- **Rust nightly** (richiesto dai crate Vortex)
- **SQLcl** installato (o specificare il percorso con `--sqlcl-path`)
- Un database Oracle accessibile

### Installazione di Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Installazione di SQLcl

Scaricare SQLcl da: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

O su Linux:
```bash
# Esempio per installare in /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Installazione

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

L'eseguibile sarà disponibile in `target/release/oracle2vortex`.

## Utilizzo

### Sintassi di base

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

### Opzioni

| Opzione | Breve | Descrizione | Predefinito |
|---------|-------|-------------|-------------|
| `--sql-file` | `-f` | Percorso del file SQL contenente la query | (richiesto) |
| `--output` | `-o` | Percorso del file Vortex di output | (richiesto) |
| `--host` | | Host Oracle | (richiesto) |
| `--port` | | Porta Oracle | 1521 |
| `--user` | `-u` | Utente Oracle | (richiesto) |
| `--password` | `-p` | Password Oracle | (richiesto) |
| `--sid` | | SID o nome servizio Oracle | (richiesto) |
| `--sqlcl-path` | | Percorso dell'eseguibile SQLcl | `sql` |
| `--auto-batch-rows` | | Numero di righe per batch (0 = disabilitato) | 0 |
| `--skip-lobs` | | Salta i tipi LOB Oracle (CLOB, BLOB, NCLOB) | false |

### Auto-Batching (Tabelle grandi)

Per elaborare tabelle con milioni o miliardi di righe con utilizzo costante della memoria, utilizzare l'opzione `--auto-batch-rows`:

```bash
# Elaborare in batch di 50000 righe
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

**Come funziona:**
1. Racchiude automaticamente la query con `OFFSET/FETCH`
2. Esegue SQLcl più volte (una volta per batch)
3. Accumula tutti i risultati in memoria
4. Scrive un unico file Vortex contenente tutti i dati

**Limitazioni:**
- Richiede Oracle 12c+ (sintassi OFFSET/FETCH)
- La query NON deve già contenere OFFSET/FETCH o ROWNUM
- Consigliato: aggiungere ORDER BY per un ordine coerente

**Memoria:** Con auto-batching, memoria utilizzata = dimensione batch × 2 (JSON + Vortex)  
Esempio: 50000 righe × 1 KB = 100 MB per batch (invece di caricare l'intera tabella)

**Vedi anche:** `BATCH_PROCESSING.md` e `README_LARGE_DATASETS.md` per maggiori dettagli.

### Saltare le colonne LOB

I tipi LOB Oracle (CLOB, BLOB, NCLOB) possono essere molto grandi e potrebbero non essere necessari per l'analisi. Utilizzare `--skip-lobs` per escluderli:

```bash
# Salta le colonne LOB per ridurre le dimensioni del file e migliorare le prestazioni
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

**Come funziona:**
- Rileva e filtra automaticamente le colonne contenenti dati LOB
- I LOB sono identificati per dimensione (> 4000 caratteri) o indicatori binari
- Il primo record registrato mostrerà quante colonne sono state saltate
- Riduce significativamente le dimensioni del file e l'utilizzo della memoria per tabelle con campi di testo/binari di grandi dimensioni

**Casi d'uso:**
- Esportare tabelle di metadati con campi di descrizione
- Lavorare con tabelle contenenti documenti XML o JSON di grandi dimensioni
- Concentrarsi sui dati strutturati ignorando il contenuto binario
- Ottimizzazione delle prestazioni per tabelle con molte colonne grandi

### Esempio con file SQL

Creare un file `query.sql`:

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

Quindi eseguire:

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

## Architettura

```
┌─────────────┐
│  File SQL   │
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
│  File .vortex            │
│  (columnar binary)       │
└──────────────────────────┘
```

## Funzionamento

1. **Lettura SQL**: Il file SQL viene caricato in memoria
2. **Avvio SQLcl**: Il processo inizia con la connessione Oracle
3. **Configurazione sessione**:
   - `SET SQLFORMAT JSON` per l'esportazione JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` per evitare problemi di locale
4. **Esecuzione query**: La query SQL viene inviata tramite stdin
5. **Cattura output**: Lettura completa dello stdout JSON
6. **Estrazione JSON**: Isolamento della struttura `{"results":[{"items":[...]}]}`
7. **Inferenza schema**: Lo schema Vortex viene dedotto automaticamente dal primo record
8. **Conversione record**: Ogni oggetto JSON viene trasformato in colonne Vortex
9. **Scrittura file**: File Vortex binario creato con sessione Tokio

## Tipi di dati supportati

La conversione dei tipi da JSON a Vortex è automatica:

| Tipo JSON | Tipo Vortex | Nullable | Note |
|-----------|-------------|----------|------|
| `null` | `Utf8` | ✅ | Inferito come stringa nullable |
| `boolean` | `Bool` | ✅ | Tramite BoolArray |
| `number` (intero) | `Primitive(I64)` | ✅ | Rilevato con `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Rilevato con `is_f64() == true` |
| `string` | `Utf8` | ✅ | Tramite VarBinArray |
| `array` | `Utf8` | ✅ | Serializzato come stringa JSON |
| `object` | `Utf8` | ✅ | Serializzato come stringa JSON |

**Nota**: Tutti i tipi sono nullable per gestire i valori Oracle NULL.

## Log e debug

L'applicazione utilizza `tracing` per i log. I messaggi vengono visualizzati su stderr con il livello di log.

I log includono:
- Connessione a Oracle
- Numero di record elaborati
- Schema inferito
- Errori e avvisi

## Verifica dei file Vortex generati

Per verificare i file generati, utilizzare lo strumento `vx`:

```bash
# Installazione di vx (strumento CLI Vortex)
cargo install vortex-vx

# Esplorare un file Vortex
vx browse output.vortex

# Visualizzare i metadati
vx info output.vortex
```

## Limitazioni e considerazioni

- **Tipi complessi**: Gli oggetti JSON nidificati e gli array vengono serializzati in stringhe
- **Buffer in memoria**: I record sono attualmente bufferizzati prima della scrittura (possibile ottimizzazione futura)
- **Schema fisso**: Inferito solo dal primo record (i record successivi devono corrispondere)
- **Sicurezza**: La password viene passata come argomento CLI (visibile con `ps`). Utilizzare variabili d'ambiente in produzione.
- **Tipi LOB**: Per impostazione predefinita, le colonne LOB (CLOB, BLOB, NCLOB) sono incluse. Utilizzare `--skip-lobs` per escluderle per migliori prestazioni e file di dimensioni inferiori.

## Sviluppo

### Build in modalità debug

```bash
cargo build
```

### Build in modalità release

```bash
cargo build --release
```

Il binario sarà in `target/release/oracle2vortex` (~46 MB in release).

### Test

```bash
cargo test
```

### Test manuali

I file di test con credenziali sono in `tests_local/` (gitignored):

```bash
# Creare query di test
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Eseguire
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licenza

Copyright (c) 2026 William Gacquer

Questo progetto è concesso in licenza sotto EUPL-1.2 (European Union Public Licence v. 1.2).

**IMPORTANTE - Restrizione d'uso commerciale:**  
L'uso commerciale di questo software è vietato senza previo accordo scritto con l'autore.  
Per qualsiasi richiesta di licenza commerciale, contattare: **oracle2vortex@amilto.com**

Vedere il file [LICENSE](LICENSE) per il testo completo della licenza.

## Autore

**William Gacquer**  
Contatto: oracle2vortex@amilto.com

## Cronologia dei test

Il progetto è stato validato su un database Oracle di produzione:

- ✅ **Test semplice**: 10 record, 3 colonne → 5,5 KB
- ✅ **Test complesso**: 100 record, 417 colonne → 1,3 MB
- ✅ **Validazione**: File leggibili con `vx browse` (Vortex v0.58)

## Struttura del progetto

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Questo file
├── IMPLEMENTATION.md       # Documentazione tecnica
├── .gitignore             # Esclude tests_local/ e credenziali
├── src/
│   ├── main.rs            # Entry point con tokio runtime
│   ├── cli.rs             # Parsing argomenti Clap
│   ├── sqlcl.rs           # Processo SQLcl con CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Conversione JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Orchestrazione completa
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Query di esempio
└── tests_local/           # Test con credenziali (gitignored)
```

## Dipendenze principali

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Risorse

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
