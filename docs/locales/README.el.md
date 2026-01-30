# oracle2vortex

Μια εφαρμογή CLI που εξάγει πίνακες Oracle σε μορφή Vortex μέσω SQLcl με streaming JSON.

## Περιγραφή

Το `oracle2vortex` επιτρέπει την εξαγωγή δεδομένων Oracle χρησιμοποιώντας:
- **SQLcl** για τη σύνδεση και την εγγενή εξαγωγή σε JSON
- **Streaming** για επεξεργασία δεδομένων εν κινήσει χωρίς αναμονή λήξης εξαγωγής
- **Αυτόματη μετατροπή** σε στηλοθετική μορφή Vortex με συμπερασμό σχήματος

✅ **Ολοκληρωμένο και δοκιμασμένο έργο σε παραγωγή** - Επικυρωμένο με πίνακα 417 στηλών σε πραγματική βάση δεδομένων.

## Προαπαιτούμενα

- **Rust nightly** (απαιτείται από τα Vortex crates)
- **SQLcl** εγκατεστημένο (ή καθορίστε τη διαδρομή με `--sqlcl-path`)
- Μια προσβάσιμη βάση δεδομένων Oracle

### Εγκατάσταση Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Εγκατάσταση SQLcl

Λήψη SQLcl από: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Ή σε Linux:
```bash
# Παράδειγμα για εγκατάσταση στο /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Εγκατάσταση

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Το εκτελέσιμο θα είναι διαθέσιμο στο `target/release/oracle2vortex`.

## Χρήση

### Βασική σύνταξη

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

### Επιλογές

| Επιλογή | Σύντομη | Περιγραφή | Προεπιλογή |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Διαδρομή προς το αρχείο SQL που περιέχει το ερώτημα | (απαιτείται) |
| `--output` | `-o` | Διαδρομή του αρχείου εξόδου Vortex | (απαιτείται) |
| `--host` | | Διακομιστής Oracle | (απαιτείται) |
| `--port` | | Θύρα Oracle | 1521 |
| `--user` | `-u` | Χρήστης Oracle | (απαιτείται) |
| `--password` | `-p` | Κωδικός πρόσβασης Oracle | (απαιτείται) |
| `--sid` | | SID ή όνομα υπηρεσίας Oracle | (απαιτείται) |
| `--sqlcl-path` | | Διαδρομή προς το εκτελέσιμο SQLcl | `sql` |
| `--auto-batch-rows` | | Αριθμός γραμμών ανά παρτίδα (0 = απενεργοποιημένο) | 0 |
| `--skip-lobs` | | Παράλειψη τύπων LOB Oracle (CLOB, BLOB, NCLOB) | false |

### Αυτόματη επεξεργασία παρτίδων (Μεγάλοι πίνακες)

Για επεξεργασία πινάκων με εκατομμύρια ή δισεκατομμύρια γραμμές με σταθερή χρήση μνήμης, χρησιμοποιήστε την επιλογή `--auto-batch-rows`:

```bash
# Επεξεργασία σε παρτίδες 50000 γραμμών
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

**Πώς λειτουργεί:**
1. Τυλίγει αυτόματα το ερώτημά σας με `OFFSET/FETCH`
2. Εκτελεί το SQLcl πολλές φορές (μία φορά ανά παρτίδα)
3. Συγκεντρώνει όλα τα αποτελέσματα στη μνήμη
4. Γράφει ένα μόνο αρχείο Vortex που περιέχει όλα τα δεδομένα

**Περιορισμοί:**
- Απαιτεί Oracle 12c+ (σύνταξη OFFSET/FETCH)
- Το ερώτημά σας ΔΕΝ πρέπει να περιέχει ήδη OFFSET/FETCH ή ROWNUM
- Συνιστάται: προσθέστε ORDER BY για συνεπή σειρά

**Μνήμη:** Με αυτόματη επεξεργασία παρτίδων, χρησιμοποιούμενη μνήμη = μέγεθος παρτίδας × 2 (JSON + Vortex)  
Παράδειγμα: 50000 γραμμές × 1 KB = 100 MB ανά παρτίδα (αντί να φορτώνεται ολόκληρος ο πίνακας)

**Δείτε επίσης:** `BATCH_PROCESSING.md` και `README_LARGE_DATASETS.md` για περισσότερες λεπτομέρειες.

### Παράλειψη στηλών LOB

Οι τύποι LOB Oracle (CLOB, BLOB, NCLOB) μπορεί να είναι πολύ μεγάλοι και μπορεί να μην είναι απαραίτητοι για ανάλυση. Χρησιμοποιήστε `--skip-lobs` για να τους εξαιρέσετε:

```bash
# Παράλειψη στηλών LOB για μείωση μεγέθους αρχείου και βελτίωση απόδοσης
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

**Πώς λειτουργεί:**
- Ανιχνεύει και φιλτράρει αυτόματα στήλες που περιέχουν δεδομένα LOB
- Τα LOB αναγνωρίζονται από το μέγεθος (> 4000 χαρακτήρες) ή δυαδικούς δείκτες
- Η πρώτη καταγεγραμμένη εγγραφή θα δείξει πόσες στήλες παραλείφθηκαν
- Μειώνει σημαντικά το μέγεθος αρχείου και τη χρήση μνήμης για πίνακες με μεγάλα πεδία κειμένου/δυαδικά

**Περιπτώσεις χρήσης:**
- Εξαγωγή πινάκων μεταδεδομένων με πεδία περιγραφής
- Εργασία με πίνακες που περιέχουν έγγραφα XML ή μεγάλα έγγραφα JSON
- Εστίαση σε δομημένα δεδομένα αγνοώντας το δυαδικό περιεχόμενο
- Βελτιστοποίηση απόδοσης για πίνακες με πολλές μεγάλες στήλες

### Παράδειγμα με αρχείο SQL

Δημιουργήστε ένα αρχείο `query.sql`:

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

Στη συνέχεια εκτελέστε:

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

## Αρχιτεκτονική

```
┌─────────────┐
│  Αρχείο     │
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
│  Αρχείο .vortex          │
│  (columnar binary)       │
└──────────────────────────┘
```

## Λειτουργία

1. **Ανάγνωση SQL**: Το αρχείο SQL φορτώνεται στη μνήμη
2. **Εκκίνηση SQLcl**: Εκκίνηση διεργασίας με σύνδεση Oracle
3. **Διαμόρφωση συνεδρίας**:
   - `SET SQLFORMAT JSON` για εξαγωγή JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` για αποφυγή προβλημάτων locale
4. **Εκτέλεση ερωτήματος**: Το ερώτημα SQL αποστέλλεται μέσω stdin
5. **Καταγραφή εξόδου**: Πλήρης ανάγνωση του stdout JSON
6. **Εξαγωγή JSON**: Απομόνωση της δομής `{"results":[{"items":[...]}]}`
7. **Συμπερασμός σχήματος**: Το σχήμα Vortex συνάγεται αυτόματα από την πρώτη εγγραφή
8. **Μετατροπή εγγραφών**: Κάθε αντικείμενο JSON μετατρέπεται σε στήλες Vortex
9. **Εγγραφή αρχείου**: Δημιουργία δυαδικού αρχείου Vortex με συνεδρία Tokio

## Υποστηριζόμενοι τύποι δεδομένων

Η μετατροπή τύπων JSON σε Vortex γίνεται αυτόματα:

| Τύπος JSON | Τύπος Vortex | Nullable | Σημειώσεις |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Συνάγεται ως nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (ακέραιος) | `Primitive(I64)` | ✅ | Ανιχνεύεται με `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Ανιχνεύεται με `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Σειριοποιημένο ως JSON string |
| `object` | `Utf8` | ✅ | Σειριοποιημένο ως JSON string |

**Σημείωση**: Όλοι οι τύποι είναι nullable για διαχείριση τιμών Oracle NULL.

## Αρχεία καταγραφής και αποσφαλμάτωση

Η εφαρμογή χρησιμοποιεί το `tracing` για αρχεία καταγραφής. Τα μηνύματα εμφανίζονται στο stderr με το επίπεδο καταγραφής.

Τα αρχεία καταγραφής περιλαμβάνουν:
- Σύνδεση στο Oracle
- Αριθμός επεξεργασμένων εγγραφών
- Συναγόμενο σχήμα
- Σφάλματα και προειδοποιήσεις

## Επαλήθευση δημιουργημένων αρχείων Vortex

Για επαλήθευση των δημιουργημένων αρχείων, χρησιμοποιήστε το εργαλείο `vx`:

```bash
# Εγκατάσταση vx (εργαλείο Vortex CLI)
cargo install vortex-vx

# Εξερεύνηση αρχείου Vortex
vx browse output.vortex

# Εμφάνιση μεταδεδομένων
vx info output.vortex
```

## Περιορισμοί και παρατηρήσεις

- **Σύνθετοι τύποι**: Τα ένθετα αντικείμενα JSON και οι πίνακες σειριοποιούνται σε συμβολοσειρές
- **Buffer στη μνήμη**: Οι εγγραφές αποθηκεύονται προσωρινά πριν την εγγραφή (πιθανή μελλοντική βελτιστοποίηση)
- **Σταθερό σχήμα**: Συνάγεται μόνο από την πρώτη εγγραφή (οι επόμενες εγγραφές πρέπει να ταιριάζουν)
- **Ασφάλεια**: Ο κωδικός πρόσβασης περνά ως όρισμα CLI (ορατός με `ps`). Χρησιμοποιήστε μεταβλητές περιβάλλοντος σε παραγωγή.
- **Τύποι LOB**: Από προεπιλογή, οι στήλες LOB (CLOB, BLOB, NCLOB) συμπεριλαμβάνονται. Χρησιμοποιήστε `--skip-lobs` για να τις εξαιρέσετε για καλύτερη απόδοση και μικρότερα μεγέθη αρχείων.

## Ανάπτυξη

### Build σε λειτουργία debug

```bash
cargo build
```

### Build σε λειτουργία release

```bash
cargo build --release
```

Το δυαδικό θα βρίσκεται στο `target/release/oracle2vortex` (~46 MB σε release).

### Δοκιμές

```bash
cargo test
```

### Χειροκίνητες δοκιμές

Τα αρχεία δοκιμών με διαπιστευτήρια βρίσκονται στο `tests_local/` (gitignored):

```bash
# Δημιουργία ερωτημάτων δοκιμής
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Εκτέλεση
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Άδεια χρήσης

Copyright (c) 2026 William Gacquer

Αυτό το έργο διατίθεται υπό την άδεια EUPL-1.2 (European Union Public Licence v. 1.2).

**ΣΗΜΑΝΤΙΚΟ - Περιορισμός εμπορικής χρήσης:**  
Η εμπορική χρήση αυτού του λογισμικού απαγορεύεται χωρίς προηγούμενη γραπτή συμφωνία με τον συγγραφέα.  
Για οποιοδήποτε αίτημα εμπορικής άδειας, επικοινωνήστε: **oracle2vortex@amilto.com**

Δείτε το αρχείο [LICENSE](LICENSE) για το πλήρες κείμενο της άδειας.

## Συγγραφέας

**William Gacquer**  
Επικοινωνία: oracle2vortex@amilto.com

## Ιστορικό δοκιμών

Το έργο επικυρώθηκε σε βάση δεδομένων Oracle παραγωγής:

- ✅ **Απλή δοκιμή**: 10 εγγραφές, 3 στήλες → 5.5 KB
- ✅ **Σύνθετη δοκιμή**: 100 εγγραφές, 417 στήλες → 1.3 MB
- ✅ **Επικύρωση**: Αρχεία αναγνώσιμα με `vx browse` (Vortex v0.58)

## Δομή έργου

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Αυτό το αρχείο
├── IMPLEMENTATION.md       # Τεχνική τεκμηρίωση
├── .gitignore             # Εξαιρεί tests_local/ και διαπιστευτήρια
├── src/
│   ├── main.rs            # Entry point με runtime tokio
│   ├── cli.rs             # Ανάλυση ορισμάτων Clap
│   ├── sqlcl.rs           # Διεργασία SQLcl με CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Μετατροπή JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Πλήρης ενορχήστρωση
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Παράδειγμα ερωτήματος
└── tests_local/           # Δοκιμές με διαπιστευτήρια (gitignored)
```

## Κύριες εξαρτήσεις

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Πόροι

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
