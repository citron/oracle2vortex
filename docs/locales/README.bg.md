# oracle2vortex

CLI приложение за експортиране на Oracle таблици във формат Vortex чрез SQLcl със стрийминг JSON.

## Описание

`oracle2vortex` позволява експортиране на данни от Oracle използвайки:
- **SQLcl** за връзка и нативен JSON експорт
- **Стрийминг** за обработка на данни в реално време без изчакване края на експорта
- **Автоматична конверсия** към колонен формат Vortex с извеждане на схема

✅ **Проектът е завършен и тестван в продукция** - Валидиран с таблица от 417 колони на реална база данни.

## Предварителни изисквания

- **Rust nightly** (изисква се от Vortex crate-ове)
- **SQLcl** инсталиран (или задайте пътя с `--sqlcl-path`)
- Достъпна Oracle база данни

### Инсталиране на Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Инсталиране на SQLcl

Изтеглете SQLcl от: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Или на Linux:
```bash
# Пример за инсталиране в /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Инсталиране

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

Изпълнимият файл ще бъде наличен в `target/release/oracle2vortex`.

## Използване

### Основен синтаксис

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

### Опции

| Опция | Кратка | Описание | По подразбиране |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | Път към SQL файла съдържащ заявката | (задължително) |
| `--output` | `-o` | Път на изходния Vortex файл | (задължително) |
| `--host` | | Oracle хост | (задължително) |
| `--port` | | Oracle порт | 1521 |
| `--user` | `-u` | Oracle потребител | (задължително) |
| `--password` | `-p` | Oracle парола | (задължително) |
| `--sid` | | Oracle SID или име на услуга | (задължително) |
| `--sqlcl-path` | | Път към изпълнимия SQLcl | `sql` |
| `--auto-batch-rows` | | Брой редове на партида (0 = изключено) | 0 |

### Авто-групова обработка (големи таблици)

За обработка на таблици с милиони или милиарди редове с постоянно използване на памет, използвайте опцията `--auto-batch-rows`:

```bash
# Обработка в партиди от 50000 реда
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

**Как работи:**
1. Автоматично обвива заявката с `OFFSET/FETCH`
2. Изпълнява SQLcl многократно (веднъж на партида)
3. Натрупва всички резултати в паметта
4. Записва един Vortex файл съдържащ всички данни

**Ограничения:**
- Изисква Oracle 12c+ (OFFSET/FETCH синтаксис)
- Вашата заявка НЕ трябва вече да съдържа OFFSET/FETCH или ROWNUM
- Препоръчително: добавете ORDER BY за последователен ред

**Памет:** С авто-групова обработка, използвана памет = размер партида × 2 (JSON + Vortex)  
Пример: 50000 реда × 1 KB = 100 MB на партида (вместо зареждане на цялата таблица)

**Вижте също:** `BATCH_PROCESSING.md` и `README_LARGE_DATASETS.md` за повече детайли.

### Пример с SQL файл

Създайте файл `query.sql`:

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

След това изпълнете:

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

## Архитектура

```
┌─────────────┐
│  SQL        │
│  файл       │
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
│  .vortex файл            │
│  (columnar binary)       │
└──────────────────────────┘
```

## Функциониране

1. **Четене на SQL**: SQL файлът се зарежда в паметта
2. **Стартиране на SQLcl**: Стартиране на процес с Oracle връзка
3. **Конфигуриране на сесия**:
   - `SET SQLFORMAT JSON` за JSON експорт
   - `SET NLS_NUMERIC_CHARACTERS='.,';` за избягване проблеми с locale
4. **Изпълнение на заявка**: SQL заявката се изпраща чрез stdin
5. **Прихващане на изход**: Пълно четене на JSON stdout
6. **Извличане на JSON**: Изолиране на структурата `{"results":[{"items":[...]}]}`
7. **Извеждане на схема**: Vortex схемата се извежда автоматично от първия запис
8. **Конвертиране на записи**: Всеки JSON обект се трансформира в Vortex колони
9. **Записване на файл**: Бинарен Vortex файл се създава с Tokio сесия

## Поддържани типове данни

Конверсията на JSON типове към Vortex става автоматично:

| JSON тип | Vortex тип | Nullable | Бележки |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Изведен като nullable string |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (цяло число) | `Primitive(I64)` | ✅ | Открит с `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Открит с `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Сериализиран като JSON string |
| `object` | `Utf8` | ✅ | Сериализиран като JSON string |

**Забележка**: Всички типове са nullable за обработка на Oracle NULL стойности.

## Логове и дебъгване

Приложението използва `tracing` за логове. Съобщенията се показват на stderr с нивото на лога.

Логовете включват:
- Връзка към Oracle
- Брой обработени записи
- Изведена схема
- Грешки и предупреждения

## Проверка на генерирани Vortex файлове

За проверка на генерираните файлове, използвайте инструмента `vx`:

```bash
# Инсталиране на vx (Vortex CLI инструмент)
cargo install vortex-vx

# Разглеждане на Vortex файл
vx browse output.vortex

# Показване на метаданни
vx info output.vortex
```

## Ограничения и съображения

- **Сложни типове**: Вложени JSON обекти и масиви се сериализират в низове
- **Буфер в памет**: Записите понастоящем се буферират преди записване (бъдеща оптимизация възможна)
- **Фиксирана схема**: Изведена само от първия запис (следващите записи трябва да съответстват)
- **Сигурност**: Паролата се подава като CLI аргумент (видима с `ps`). Използвайте променливи на средата в продукция.

## Разработка

### Build в debug режим

```bash
cargo build
```

### Build в release режим

```bash
cargo build --release
```

Бинарният файл ще бъде в `target/release/oracle2vortex` (~46 MB в release).

### Тестове

```bash
cargo test
```

### Ръчни тестове

Тестовите файлове с идентификационни данни са в `tests_local/` (gitignored):

```bash
# Създаване на тестови заявки
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Изпълнение
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Лиценз

Copyright (c) 2026 William Gacquer

Този проект е лицензиран под EUPL-1.2 (European Union Public Licence v. 1.2).

**ВАЖНО - Ограничение за търговска употреба:**  
Търговската употреба на този софтуер е забранена без предварително писмено съгласие от автора.  
За всякакви запитвания за търговски лиценз, свържете се: **oracle2vortex@amilto.com**

Вижте файла [LICENSE](LICENSE) за пълния текст на лиценза.

## Автор

**William Gacquer**  
Контакт: oracle2vortex@amilto.com

## История на тестовете

Проектът е валидиран на продукционна Oracle база данни:

- ✅ **Прост тест**: 10 записа, 3 колони → 5.5 KB
- ✅ **Сложен тест**: 100 записа, 417 колони → 1.3 MB
- ✅ **Валидиране**: Файлове четими с `vx browse` (Vortex v0.58)

## Структура на проекта

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Този файл
├── IMPLEMENTATION.md       # Техническа документация
├── .gitignore             # Изключва tests_local/ и идентификационни данни
├── src/
│   ├── main.rs            # Entry point с tokio runtime
│   ├── cli.rs             # Обработка на Clap аргументи
│   ├── sqlcl.rs           # SQLcl процес с CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Конверсия JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Пълна оркестрация
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Примерна заявка
└── tests_local/           # Тестове с идентификационни данни (gitignored)
```

## Основни зависимости

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Ресурси

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
