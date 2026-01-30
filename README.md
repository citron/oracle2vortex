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
| `--host` | | Hôte Oracle | (requis) |
| `--port` | | Port Oracle | 1521 |
| `--user` | `-u` | Utilisateur Oracle | (requis) |
| `--password` | `-p` | Mot de passe Oracle | (requis) |
| `--sid` | | SID ou nom de service Oracle | (requis) |
| `--sqlcl-path` | | Chemin vers l'exécutable SQLcl | `sql` |
| `--auto-batch-rows` | | Nombre de lignes par lot (0 = désactivé) | 0 |

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

La conversion des types JSON vers Vortex se fait automatiquement :

| Type JSON | Type Vortex | Nullable | Notes |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Déduit comme string nullable |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (entier) | `Primitive(I64)` | ✅ | Détecté avec `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Détecté avec `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Sérialisé comme string JSON |
| `object` | `Utf8` | ✅ | Sérialisé comme string JSON |

**Note** : Tous les types sont nullable pour gérer les valeurs Oracle NULL.

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

## Limitations et considérations

- **Types complexes** : Les objets JSON imbriqués et les tableaux sont sérialisés en chaînes
- **Buffer en mémoire** : Les records sont actuellement bufferisés avant écriture (optimisation future possible)
- **Schéma fixe** : Inféré du premier record uniquement (les records suivants doivent correspondre)
- **Sécurité** : Le mot de passe est passé en argument CLI (visible avec `ps`). Utiliser des variables d'environnement en production.

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

[À définir]

## Contributeurs

[À définir]

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
