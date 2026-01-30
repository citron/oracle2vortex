# oracle2vortex

Uma aplicação CLI que extrai tabelas Oracle para o formato Vortex via SQLcl com streaming JSON.

## Descrição

`oracle2vortex` permite exportar dados Oracle usando:
- **SQLcl** para conexão e exportação nativa em JSON
- **Streaming** para processar dados em tempo real sem esperar pela conclusão da exportação
- **Conversão automática** para o formato colunar Vortex com inferência de esquema

✅ **Projeto concluído e testado em produção** - Validado com uma tabela de 417 colunas em base de dados real.

## Pré-requisitos

- **Rust nightly** (requerido pelos crates Vortex)
- **SQLcl** instalado (ou especificar o caminho com `--sqlcl-path`)
- Uma base de dados Oracle acessível

### Instalação do Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### Instalação do SQLcl

Transferir o SQLcl de: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

Ou no Linux:
```bash
# Exemplo para instalar em /opt/oracle/sqlcl/
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## Instalação

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

O executável estará disponível em `target/release/oracle2vortex`.

## Utilização

### Sintaxe básica

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

### Opções

| Opção | Curta | Descrição | Padrão |
|-------|-------|-----------|--------|
| `--sql-file` | `-f` | Caminho para o ficheiro SQL contendo a consulta | (requerido) |
| `--output` | `-o` | Caminho do ficheiro Vortex de saída | (requerido) |
| `--host` | | Host Oracle | (requerido) |
| `--port` | | Porta Oracle | 1521 |
| `--user` | `-u` | Utilizador Oracle | (requerido) |
| `--password` | `-p` | Palavra-passe Oracle | (requerido) |
| `--sid` | | SID ou nome de serviço Oracle | (requerido) |
| `--sqlcl-path` | | Caminho para o executável SQLcl | `sql` |
| `--auto-batch-rows` | | Número de linhas por lote (0 = desativado) | 0 |

### Auto-Batching (Tabelas Grandes)

Para processar tabelas com milhões ou milhares de milhões de linhas com utilização constante de memória, use a opção `--auto-batch-rows`:

```bash
# Processar em lotes de 50000 linhas
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

**Como funciona:**
1. Envolve automaticamente a sua consulta com `OFFSET/FETCH`
2. Executa SQLcl várias vezes (uma vez por lote)
3. Acumula todos os resultados em memória
4. Escreve um único ficheiro Vortex contendo todos os dados

**Limitações:**
- Requer Oracle 12c+ (sintaxe OFFSET/FETCH)
- A sua consulta NÃO deve já conter OFFSET/FETCH ou ROWNUM
- Recomendado: adicionar ORDER BY para ordem consistente

**Memória:** Com auto-batching, memória utilizada = tamanho do lote × 2 (JSON + Vortex)  
Exemplo: 50000 linhas × 1 KB = 100 MB por lote (em vez de carregar toda a tabela)

**Ver também:** `BATCH_PROCESSING.md` e `README_LARGE_DATASETS.md` para mais detalhes.

### Exemplo com ficheiro SQL

Crie um ficheiro `query.sql`:

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

Depois execute:

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

## Arquitetura

```
┌─────────────┐
│  Ficheiro   │
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
│  Ficheiro .vortex        │
│  (columnar binary)       │
└──────────────────────────┘
```

## Funcionamento

1. **Leitura SQL**: O ficheiro SQL é carregado na memória
2. **Lançamento SQLcl**: Início do processo com conexão Oracle
3. **Configuração de sessão**:
   - `SET SQLFORMAT JSON` para exportação JSON
   - `SET NLS_NUMERIC_CHARACTERS='.,';` para evitar problemas de locale
4. **Execução da consulta**: A consulta SQL é enviada via stdin
5. **Captura de saída**: Leitura completa do stdout JSON
6. **Extração JSON**: Isolamento da estrutura `{"results":[{"items":[...]}]}`
7. **Inferência de esquema**: O esquema Vortex é deduzido automaticamente do primeiro registo
8. **Conversão de registos**: Cada objeto JSON é transformado em colunas Vortex
9. **Escrita de ficheiro**: Ficheiro Vortex binário criado com sessão Tokio

## Tipos de dados suportados

A conversão de tipos JSON para Vortex é automática:

| Tipo JSON | Tipo Vortex | Nullable | Notas |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | Inferido como string nullable |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (inteiro) | `Primitive(I64)` | ✅ | Detetado com `is_f64() == false` |
| `number` (float) | `Primitive(F64)` | ✅ | Detetado com `is_f64() == true` |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | Serializado como string JSON |
| `object` | `Utf8` | ✅ | Serializado como string JSON |

**Nota**: Todos os tipos são nullable para tratar valores Oracle NULL.

## Logs e depuração

A aplicação usa `tracing` para logs. As mensagens são apresentadas em stderr com o nível de log.

Os logs incluem:
- Conexão a Oracle
- Número de registos processados
- Esquema inferido
- Erros e avisos

## Verificação de ficheiros Vortex gerados

Para verificar os ficheiros gerados, use a ferramenta `vx`:

```bash
# Instalação do vx (ferramenta CLI Vortex)
cargo install vortex-vx

# Explorar um ficheiro Vortex
vx browse output.vortex

# Apresentar metadados
vx info output.vortex
```

## Limitações e considerações

- **Tipos complexos**: Objetos JSON aninhados e arrays são serializados em strings
- **Buffer em memória**: Os registos são atualmente armazenados em buffer antes da escrita (otimização futura possível)
- **Esquema fixo**: Inferido apenas do primeiro registo (registos subsequentes devem corresponder)
- **Segurança**: A palavra-passe é passada como argumento CLI (visível com `ps`). Use variáveis de ambiente em produção.

## Desenvolvimento

### Build em modo debug

```bash
cargo build
```

### Build em modo release

```bash
cargo build --release
```

O binário estará em `target/release/oracle2vortex` (~46 MB em release).

### Testes

```bash
cargo test
```

### Testes manuais

Os ficheiros de teste com credenciais estão em `tests_local/` (gitignored):

```bash
# Criar consultas de teste
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# Executar
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## Licença

Copyright (c) 2026 William Gacquer

Este projeto está licenciado sob EUPL-1.2 (European Union Public Licence v. 1.2).

**IMPORTANTE - Restrição de uso comercial:**  
O uso comercial deste software é proibido sem acordo prévio por escrito com o autor.  
Para qualquer pedido de licença comercial, contacte: **oracle2vortex@amilto.com**

Veja o ficheiro [LICENSE](LICENSE) para o texto completo da licença.

## Autor

**William Gacquer**  
Contacto: oracle2vortex@amilto.com

## Histórico de testes

O projeto foi validado numa base de dados Oracle de produção:

- ✅ **Teste simples**: 10 registos, 3 colunas → 5,5 KB
- ✅ **Teste complexo**: 100 registos, 417 colunas → 1,3 MB
- ✅ **Validação**: Ficheiros legíveis com `vx browse` (Vortex v0.58)

## Estrutura do projeto

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # Este ficheiro
├── IMPLEMENTATION.md       # Documentação técnica
├── .gitignore             # Exclui tests_local/ e credenciais
├── src/
│   ├── main.rs            # Entry point com tokio runtime
│   ├── cli.rs             # Parsing de argumentos Clap
│   ├── sqlcl.rs           # Processo SQLcl com CONNECT
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # Conversão JSON→Vortex (API 0.58)
│   └── pipeline.rs        # Orquestração completa
├── examples/
│   ├── README.md
│   └── sample_query.sql   # Consulta de exemplo
└── tests_local/           # Testes com credenciais (gitignored)
```

## Dependências principais

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
