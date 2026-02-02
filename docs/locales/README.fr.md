# oracle2vortex

Une application CLI qui extrait des tables Oracle vers le format Vortex via SQLcl avec streaming JSON.

## Description

`oracle2vortex` permet d'exporter des données Oracle en utilisant :
- **SQLcl** pour la connexion et l'export natif en JSON
- **Streaming** pour traiter les données à la volée sans attendre la fin de l'export
- **Conversion automatique** vers le format Vortex columnaire avec inférence de schéma

✅ **Projet terminé et testé en production** - Validé avec une table de 417 colonnes sur base réelle.

## Prérequis

- **Rust nightly** (requis par les crates Vortex)
- **SQLcl** installé (ou spécifier le chemin avec `--sqlcl-path`)
- Une base de données Oracle accessible

### Installation de Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Installation de SQLcl

Télécharger SQLcl depuis : https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Ou sur Linux :
```bash
# Exemple pour installer dans /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Installation

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

L'exécutable sera disponible dans `target/release/oracle2vortex`.

## Utilisation

### Syntaxe de base

**Option 1 : Utiliser une chaîne de connexion (recommandé)**

```bash
oracle2vortex \
  --sql-file query.sql \
  --output data.vortex \
  --connect-string "hr/mypassword@//localhost:1521/ORCL"
```

**Option 2 : Utiliser les composants individuels**

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

### Options

| Option | Courte | Description | Défaut |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Chemin vers le fichier SQL contenant la requête | (requis) |
| `--output` | `-o` | Chemin du fichier Vortex de sortie | (requis) |
| `--connect-string` | `-c` | Chaîne de connexion Oracle complète (user/password@identifier) | - |
| `--host` | | Hôte Oracle (requis si pas de --connect-string) | - |
| `--port` | | Port Oracle | 1521 |
| `--user` | `-u` | Utilisateur Oracle (requis si pas de --connect-string) | - |
| `--password` | `-p` | Mot de passe Oracle (requis si pas de --connect-string) | - |
| `--sid` | | SID ou nom de service Oracle (requis si pas de --connect-string) | - |
| `--sqlcl-path` | | Chemin vers l'exécutable SQLcl | `sql` |
| `--auto-batch-rows` | | Nombre de lignes par lot (0 = désactivé) | 0 |
| `--skip-lobs` | | Ignorer les types LOB Oracle (CLOB, BLOB, NCLOB) | false |
| `--thick` | | Utiliser le driver Oracle Thick (JDBC/OCI) au lieu de Thin | false |

**Note** : Vous pouvez utiliser soit `--connect-string` SOIT les composants individuels (`--user`, `--password`, `--host`, `--sid`), mais pas les deux.

### Auto-Batching (Grandes Tables)

Pour traiter des tables avec des millions ou milliards de lignes avec une utilisation mémoire constante, utilisez l'option `--auto-batch-rows` :

```bash
# Traiter par lots de 50000 lignes
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

**Comment ça fonctionne :**
1. Enveloppe automatiquement votre requête avec `OFFSET/FETCH`
2. Exécute SQLcl plusieurs fois (une fois par lot)
3. Accumule tous les résultats en mémoire
4. Écrit un seul fichier Vortex contenant toutes les données

**Limites :**
- Nécessite Oracle 12c+ (syntaxe OFFSET/FETCH)
- Votre requête ne doit PAS déjà contenir OFFSET/FETCH ou ROWNUM
- Recommandé : ajouter ORDER BY pour un ordre cohérent

**Mémoire :** Avec auto-batching, la mémoire utilisée = taille d'un lot × 2 (JSON + Vortex)  
Exemple : 50000 lignes × 1 KB = 100 MB par lot (au lieu de charger toute la table)

**Voir aussi :** `BATCH_PROCESSING.md` et `README_LARGE_DATASETS.md` pour plus de détails.

### Ignorer les colonnes LOB

Les types LOB Oracle (CLOB, BLOB, NCLOB) peuvent être très volumineux et ne sont pas toujours nécessaires pour l'analyse. Utilisez `--skip-lobs` pour les exclure :

```bash
# Ignorer les colonnes LOB pour réduire la taille du fichier et améliorer les performances
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

**Fonctionnement :**
- Détecte et filtre automatiquement les colonnes contenant des données LOB
- Les LOB sont identifiés par leur taille (> 4000 caractères) ou des indicateurs binaires
- Le premier enregistrement loggé indiquera combien de colonnes ont été ignorées
- Réduit significativement la taille du fichier et l'utilisation mémoire pour les tables avec de grands champs texte/binaires

**Cas d'usage :**
- Export de tables de métadonnées avec champs de description
- Travail avec tables contenant des documents XML ou JSON volumineux
- Se concentrer sur les données structurées en ignorant le contenu binaire
- Optimisation des performances pour tables avec de nombreuses grandes colonnes

### Exemple avec fichier SQL

Créez un fichier `query.sql` :

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

Puis exécutez :

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

## Architecture

```
┌─────────────┐
│  Fichier    │
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
│  Fichier .vortex         │
│  (columnar binary)       │
└──────────────────────────┘
```

## Fonctionnement

1. **Lecture SQL** : Le fichier SQL est chargé en mémoire
2. **Lancement SQLcl** : Démarrage du process avec connexion Oracle
3. **Configuration session** :
   - `SET SQLFORMAT JSON` pour export JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` pour éviter les problèmes de locale
4. **Exécution requête** : La requête SQL est envoyée via stdin
5. **Capture sortie** : Lecture complète du stdout JSON
6. **Extraction JSON** : Isolation de la structure `{"results":[{"items":[...]}]}`
7. **Inférence schéma** : Le schéma Vortex est déduit automatiquement du premier record
8. **Conversion records** : Chaque objet JSON est transformé en colonnes Vortex
9. **Écriture fichier** : Fichier Vortex binaire créé avec session Tokio

## Types de données supportés

Mappage automatique des types Oracle vers Vortex avec stockage optimal :

### Mappage complet des types

| Type Oracle | Export JSON | Type Vortex | Stockage | Notes |
|-------------|-------------|-------------|---------|-------|
| **Types Temporels** |
| `DATE` | `"2024-01-15"` | `Extension(Date)` | I32 | Jours depuis 1970-01-01 |
| `TIMESTAMP` | `"2024-01-15T14:30:45.123456"` | `Extension(Timestamp)` | I64 | Microsecondes depuis epoch |
| `TIMESTAMP WITH TIME ZONE` | `"2024-01-15T14:30:45.123456 +02:00"` | `Extension(Timestamp)` | I64 | Converti en UTC, fuseau dans métadonnées |
| `TIMESTAMP WITH LOCAL TZ` | Identique à TIMESTAMP WITH TZ | `Extension(Timestamp)` | I64 | Converti vers fuseau session puis UTC |
| `INTERVAL DAY TO SECOND` | `"+02 02:30:00.123456"` | `Primitive(I64)` | I64 | Microsecondes totales |
| `INTERVAL YEAR TO MONTH` | `"+01-06"` | `Primitive(I32)` | I32 | Mois totaux |
| **Types Numériques** |
| `NUMBER` (entier) | `123` | `Primitive(I64)` | I64 | Nombres entiers |
| `NUMBER` (décimal) | `123.45` | `Primitive(F64)` | F64 | Virgule flottante |
| `BINARY_FLOAT` | `3.14` | `Primitive(F64)` | F64 | Précision simple IEEE 754 |
| `BINARY_DOUBLE` | `2.718` | `Primitive(F64)` | F64 | Précision double IEEE 754 |
| **Types Caractères** |
| `VARCHAR2`, `NVARCHAR2` | `"texte"` | `Utf8` | VarBinArray | Chaînes longueur variable |
| `CHAR`, `NCHAR` | `"texte"` | `Utf8` | VarBinArray | Longueur fixe (avec padding) |
| `CLOB`, `NCLOB` | `"long texte"` | `Utf8` ou ignoré | VarBinArray | Utiliser `--skip-lobs` pour exclure |
| **Types Binaires** |
| `RAW`, `LONG RAW` | `"DEADBEEF"` (hex) | `Binary` | VarBinArray | Détecté si ≥8 caractères hex |
| `BLOB` | `"chaîne hex"` | `Binary` ou ignoré | VarBinArray | Utiliser `--skip-lobs` pour exclure |
| **Types Structurés** |
| `JSON` (Oracle 21c+) | `"{\"key\":\"value\"}"` | `Utf8` | VarBinArray | JSON validé, conservé en chaîne |
| `XMLTYPE` | `"<root/>"` | `Utf8` | VarBinArray | XML en chaîne |
| **Autres Types** |
| `ROWID`, `UROWID` | `"AAABbbCCC..."` | `Utf8` | VarBinArray | Format spécifique Oracle |
| `BOOLEAN` (via JSON) | `true`/`false` | `Bool` | BitBuffer | Booléen natif |
| `null` | `null` | (inféré) | - | Variante nullable du type détecté |

**Note** : Tous les types sont nullable pour gérer les valeurs Oracle NULL.

**Pour les algorithmes détaillés de mappage et la logique de détection, voir :**
- [`docs/ORACLE_TYPE_MAPPING.md`](../ORACLE_TYPE_MAPPING.md) - Référence technique complète avec algorithmes de détection
- [`docs/TEMPORAL_TYPES.md`](../TEMPORAL_TYPES.md) - Détails d'implémentation des types temporels et tests

### Types temporels avec support des fuseaux horaires

Les colonnes temporelles Oracle sont automatiquement détectées et converties vers les types temporels natifs Vortex :

- **DATE** (YYYY-MM-DD): Stocké comme `Extension(vortex.date)` avec I32 (jours depuis 1970-01-01)
- **TIMESTAMP** (YYYY-MM-DDTHH:MI:SS[.ffffff]): Stocké comme `Extension(vortex.timestamp)` avec I64 (microsecondes depuis epoch)
- **TIMESTAMP WITH TIME ZONE**: Stocké comme `Extension(vortex.timestamp)` avec métadonnées de fuseau, **converti en UTC** pour le stockage

SQLcl est configuré pour exporter ces formats en utilisant :
```sql
ALTER SESSION SET NLS_DATE_FORMAT = 'YYYY-MM-DD"T"HH24:MI:SS';
ALTER SESSION SET NLS_TIMESTAMP_FORMAT = 'YYYY-MM-DD"T"HH24:MI:SS.FF';
ALTER SESSION SET NLS_TIMESTAMP_TZ_FORMAT = 'YYYY-MM-DD"T"HH24:MI:SS.FF TZH:TZM';
```

### Données binaires (RAW/BLOB)

Les types Oracle RAW et BLOB sont détectés lorsqu'exportés comme chaînes hexadécimales (minimum 8 caractères, majuscules) :
- Conversion automatique hex vers binaire
- Stockage efficace dans `DType::Binary` utilisant `VarBinArray`
- Exemple : `HEXTORAW('DEADBEEF')` → binaire `[0xDE, 0xAD, 0xBE, 0xEF]`

Cela garantit que les dates, horodatages et données binaires sont préservés en tant que données typées, non comme chaînes, permettant des requêtes et opérations efficaces.

## Logs et débogage

L'application utilise `tracing` pour les logs. Les messages sont affichés sur stderr avec le niveau de log.

Les logs incluent :
- Connexion à Oracle
- Nombre d'enregistrements traités
- Schéma inféré
- Erreurs et avertissements

## Vérification des fichiers Vortex générés

Pour vérifier les fichiers générés, utilisez l'outil `vx` :

```bash
# Installation de vx (outil Vortex CLI)
cargo install vortex-vx

# Explorer un fichier Vortex
vx browse output.vortex

# Afficher les métadonnées
vx info output.vortex
```

### Utiliser DuckDB

DuckDB 1.4.4+ peut lire les fichiers Vortex directement. Cependant, **les types temporels (DATE, TIMESTAMP) apparaissent comme des nombres** car DuckDB ne reconnaît pas automatiquement les types d'extension Vortex.

```sql
-- Voir les dates/timestamps comme des nombres (format de stockage brut)
SELECT * FROM 'employees.vortex';

-- Convertir en dates et timestamps lisibles
SELECT 
    employee_id,
    first_name,
    (DATE '1970-01-01' + INTERVAL (hire_date) DAYS) AS hire_date_readable,
    to_timestamp(last_update / 1000000.0) AS last_update_readable
FROM 'employees.vortex';
```

**📖 Pour le guide complet d'utilisation DuckDB, voir :** [`docs/DUCKDB_USAGE.md`](../DUCKDB_USAGE.md)

Comprend :
- Formules de conversion date/timestamp
- Gestion des types INTERVAL
- Conseils d'optimisation des performances
- Création de vues lisibles
- Stratégies de filtrage

## Limitations et considérations

- **Types complexes** : Les objets JSON imbriqués et les tableaux sont sérialisés en chaînes
- **Buffer en mémoire** : Les records sont actuellement bufferisés avant écriture (optimisation future possible)
- **Schéma fixe** : Inféré du premier record uniquement (les records suivants doivent correspondre)
- **Sécurité** : Le mot de passe est passé en argument CLI (visible avec `ps`). Utiliser des variables d'environnement en production.
- **Types LOB** : Par défaut, les colonnes LOB (CLOB, BLOB, NCLOB) sont incluses. Utilisez `--skip-lobs` pour les exclure et améliorer les performances et réduire la taille des fichiers.

## Développement

### Build en mode debug

```bash
cargo build
```

### Build en mode release

```bash
cargo build --release
```

Le binaire sera dans `target/release/oracle2vortex` (~46 MB en release).

### Tests

```bash
cargo test
```

### Tests manuels

Les fichiers de test avec credentials sont dans `tests_local/` (gitignored) :

```bash
# Créer des requêtes de test
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Exécuter
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licence

Copyright (c) 2026 William Gacquer

Ce projet est sous licence EUPL-1.2 (European Union Public Licence v. 1.2).

**IMPORTANT - Restriction d'usage commercial :**  
L'utilisation commerciale de ce logiciel est interdite sans accord écrit préalable avec l'auteur.  
Pour toute demande de licence commerciale, veuillez contacter : **oracle2vortex@amilto.com**

Voir le fichier [LICENSE](LICENSE) pour le texte complet de la licence.

## Auteur

**William Gacquer**  
Contact : oracle2vortex@amilto.com

## Historique des tests

Le projet a été validé sur une base Oracle de production :

- ✅ **Test simple** : 10 records, 3 colonnes → 5.5 KB
- ✅ **Test complexe** : 100 records, 417 colonnes → 1.3 MB
- ✅ **Validation** : Fichiers lisibles avec `vx browse` (Vortex v0.58)

## Structure du projet

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Ce fichier
├── IMPLEMENTATION.md       # Documentation technique
├── .gitignore             # Exclut tests_local/ et credentials
├── src/
│   ├── main.rs            # Entry point avec runtime tokio
│   ├── cli.rs             # Parsing arguments Clap
│   ├── sqlcl.rs           # Process SQLcl avec CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Conversion JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Orchestration complète
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Exemple de requête
└── tests_local/           # Tests avec credentials (gitignored)
```

## Dépendances principales

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Ressources

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
