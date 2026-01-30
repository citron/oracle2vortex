# oracle2vortex

Una aplicación CLI que extrae tablas Oracle al formato Vortex mediante SQLcl con transmisión JSON.

## Descripción

`oracle2vortex` permite exportar datos de Oracle usando:
- **SQLcl** para la conexión y exportación nativa en JSON
- **Streaming** para procesar datos sobre la marcha sin esperar a que finalice la exportación
- **Conversión automática** al formato columnar Vortex con inferencia de esquema

✅ **Proyecto completado y probado en producción** - Validado con una tabla de 417 columnas en una base de datos real.

## Requisitos previos

- **Rust nightly** (requerido por los crates de Vortex)
- **SQLcl** instalado (o especificar la ruta con `--sqlcl-path`)
- Una base de datos Oracle accesible

### Instalación de Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Instalación de SQLcl

Descargar SQLcl desde: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

O en Linux:
```bash
# Ejemplo para instalar en /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Instalación

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

El ejecutable estará disponible en `target/release/oracle2vortex`.

## Uso

### Sintaxis básica

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

### Opciones

| Opción | Corta | Descripción | Por defecto |
|--------|-------|-------------|-------------|
| `--sql-file` | `-f` | Ruta al archivo SQL que contiene la consulta | (requerido) |
| `--output` | `-o` | Ruta del archivo Vortex de salida | (requerido) |
| `--host` | | Host de Oracle | (requerido) |
| `--port` | | Puerto de Oracle | 1521 |
| `--user` | `-u` | Usuario de Oracle | (requerido) |
| `--password` | `-p` | Contraseña de Oracle | (requerido) |
| `--sid` | | SID o nombre de servicio de Oracle | (requerido) |
| `--sqlcl-path` | | Ruta al ejecutable SQLcl | `sql` |
| `--auto-batch-rows` | | Número de filas por lote (0 = desactivado) | 0 |

### Auto-Batching (Tablas grandes)

Para procesar tablas con millones o miles de millones de filas con uso constante de memoria, use la opción `--auto-batch-rows`:

```bash
# Procesar en lotes de 50000 filas
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

**Cómo funciona:**
1. Envuelve automáticamente su consulta con `OFFSET/FETCH`
2. Ejecuta SQLcl varias veces (una vez por lote)
3. Acumula todos los resultados en memoria
4. Escribe un único archivo Vortex que contiene todos los datos

**Limitaciones:**
- Requiere Oracle 12c+ (sintaxis OFFSET/FETCH)
- Su consulta NO debe contener ya OFFSET/FETCH o ROWNUM
- Recomendado: agregar ORDER BY para un orden coherente

**Memoria:** Con auto-batching, memoria utilizada = tamaño del lote × 2 (JSON + Vortex)  
Ejemplo: 50000 filas × 1 KB = 100 MB por lote (en lugar de cargar toda la tabla)

**Véase también:** `BATCH_PROCESSING.md` y `README_LARGE_DATASETS.md` para más detalles.

### Ejemplo con archivo SQL

Cree un archivo `query.sql`:

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

Luego ejecute:

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

## Arquitectura

```
┌─────────────┐
│  Archivo    │
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
│  Archivo .vortex         │
│  (columnar binary)       │
└──────────────────────────┘
```

## Funcionamiento

1. **Lectura SQL**: El archivo SQL se carga en memoria
2. **Inicio de SQLcl**: El proceso comienza con la conexión a Oracle
3. **Configuración de sesión**:
   - `SET SQLFORMAT JSON` para exportación JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` para evitar problemas de configuración regional
4. **Ejecución de consulta**: La consulta SQL se envía a través de stdin
5. **Captura de salida**: Lectura completa del stdout JSON
6. **Extracción JSON**: Aislamiento de la estructura `{"results":[{"items":[...]}]}`
7. **Inferencia de esquema**: El esquema Vortex se deduce automáticamente del primer registro
8. **Conversión de registros**: Cada objeto JSON se transforma en columnas Vortex
9. **Escritura de archivo**: Se crea un archivo Vortex binario con sesión Tokio

## Tipos de datos soportados

La conversión de tipos JSON a Vortex es automática:

| Tipo JSON | Tipo Vortex | Nullable | Notas |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Inferido como cadena nullable |
| `boolean` | `Bool` | ✅ | Vía BoolArray |
| `number` (entero) | `Primitive(I64)` | ✅ | Detectado con `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Detectado con `is_f64() == true` |
| `string` | `Utf8` | ✅ | Vía VarBinArray |
| `array` | `Utf8` | ✅ | Serializado como cadena JSON |
| `object` | `Utf8` | ✅ | Serializado como cadena JSON |

**Nota**: Todos los tipos son nullable para manejar valores Oracle NULL.

## Registros y depuración

La aplicación utiliza `tracing` para los registros. Los mensajes se muestran en stderr con el nivel de registro.

Los registros incluyen:
- Conexión a Oracle
- Número de registros procesados
- Esquema inferido
- Errores y advertencias

## Verificación de archivos Vortex generados

Para verificar los archivos generados, use la herramienta `vx`:

```bash
# Instalar vx (herramienta CLI de Vortex)
cargo install vortex-vx

# Explorar un archivo Vortex
vx browse output.vortex

# Mostrar metadatos
vx info output.vortex
```

## Limitaciones y consideraciones

- **Tipos complejos**: Los objetos JSON anidados y los arrays se serializan como cadenas
- **Búfer en memoria**: Los registros se almacenan actualmente en búfer antes de escribirse (posible optimización futura)
- **Esquema fijo**: Inferido solo del primer registro (los registros posteriores deben coincidir)
- **Seguridad**: La contraseña se pasa como argumento CLI (visible con `ps`). Use variables de entorno en producción.

## Desarrollo

### Compilación en modo debug

```bash
cargo build
```

### Compilación en modo release

```bash
cargo build --release
```

El binario estará en `target/release/oracle2vortex` (~46 MB en release).

### Pruebas

```bash
cargo test
```

### Pruebas manuales

Los archivos de prueba con credenciales están en `tests_local/` (gitignored):

```bash
# Crear consultas de prueba
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Ejecutar
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licencia

Copyright (c) 2026 William Gacquer

Este proyecto está licenciado bajo EUPL-1.2 (European Union Public Licence v. 1.2).

**IMPORTANTE - Restricción de uso comercial:**  
El uso comercial de este software está prohibido sin acuerdo previo por escrito con el autor.  
Para cualquier solicitud de licencia comercial, póngase en contacto con: **oracle2vortex@amilto.com**

Vea el archivo [LICENSE](LICENSE) para el texto completo de la licencia.

## Autor

**William Gacquer**  
Contacto: oracle2vortex@amilto.com

## Historial de pruebas

El proyecto ha sido validado en una base de datos Oracle de producción:

- ✅ **Prueba simple**: 10 registros, 3 columnas → 5,5 KB
- ✅ **Prueba compleja**: 100 registros, 417 columnas → 1,3 MB
- ✅ **Validación**: Archivos legibles con `vx browse` (Vortex v0.58)

## Estructura del proyecto

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Este archivo
├── IMPLEMENTATION.md       # Documentación técnica
├── .gitignore             # Excluye tests_local/ y credenciales
├── src/
│   ├── main.rs            # Punto de entrada con tokio runtime
│   ├── cli.rs             # Análisis de argumentos Clap
│   ├── sqlcl.rs           # Proceso SQLcl con CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Conversión JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Orquestación completa
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Consulta de ejemplo
└── tests_local/           # Pruebas con credenciales (gitignored)
```

## Dependencias principales

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## Recursos

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
