# oracle2vortex

通过 SQLcl 将 Oracle 表导出为 Vortex 格式的 CLI 应用程序，支持 JSON 流式传输。

## 描述

`oracle2vortex` 允许使用以下方式导出 Oracle 数据:
- **SQLcl** 用于连接和原生 JSON 导出
- **流式传输** 用于即时处理数据，无需等待导出完成
- **自动转换** 为列式 Vortex 格式，带有模式推断

✅ **项目已完成并在生产环境中测试** - 在真实数据库上使用 417 列表验证。

## 先决条件

- **Rust nightly** (Vortex crates 需要)
- **SQLcl** 已安装 (或使用 `--sqlcl-path` 指定路径)
- 可访问的 Oracle 数据库

### 安装 Rust nightly

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### 安装 SQLcl

从以下位置下载 SQLcl: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

或在 Linux 上:
```bash
# 安装到 /opt/oracle/sqlcl/ 的示例
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## 安装

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

可执行文件将位于 `target/release/oracle2vortex`。

## 使用方法

### 基本语法

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

### 选项

| 选项 | 缩写 | 描述 | 默认值 |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | 包含查询的 SQL 文件路径 | (必需) |
| `--output` | `-o` | 输出 Vortex 文件路径 | (必需) |
| `--host` | | Oracle 主机 | (必需) |
| `--port` | | Oracle 端口 | 1521 |
| `--user` | `-u` | Oracle 用户 | (必需) |
| `--password` | `-p` | Oracle 密码 | (必需) |
| `--sid` | | Oracle SID 或服务名称 | (必需) |
| `--sqlcl-path` | | SQLcl 可执行文件路径 | `sql` |
| `--auto-batch-rows` | | 每批次的行数 (0 = 禁用) | 0 |
| `--skip-lobs` | | 跳过 Oracle LOB 类型 (CLOB, BLOB, NCLOB) | false |

### 自动批处理 (大型表)

要使用恒定内存使用量处理包含数百万或数十亿行的表，请使用 `--auto-batch-rows` 选项:

```bash
# 以 50000 行为批次进行处理
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

**工作原理:**
1. 自动用 `OFFSET/FETCH` 包装您的查询
2. 多次执行 SQLcl (每批次一次)
3. 在内存中累积所有结果
4. 写入包含所有数据的单个 Vortex 文件

**限制:**
- 需要 Oracle 12c+ (OFFSET/FETCH 语法)
- 您的查询不得已包含 OFFSET/FETCH 或 ROWNUM
- 建议: 添加 ORDER BY 以确保一致的顺序

**内存:** 使用自动批处理，使用的内存 = 批次大小 × 2 (JSON + Vortex)  
示例: 50000 行 × 1 KB = 每批次 100 MB (而不是加载整个表)

**另请参阅:** `BATCH_PROCESSING.md` 和 `README_LARGE_DATASETS.md` 了解更多详细信息。

### 跳过 LOB 列

Oracle LOB 类型 (CLOB, BLOB, NCLOB) 可能非常大，并且分析时可能不需要。使用 `--skip-lobs` 排除它们:

```bash
# 跳过 LOB 列以减小文件大小并提高性能
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

**工作原理:**
- 自动检测并过滤包含 LOB 数据的列
- LOB 通过大小 (> 4000 字符) 或二进制指示符识别
- 第一条记录的日志将显示跳过了多少列
- 对于具有大型文本/二进制字段的表，可显著减少文件大小和内存使用量

**使用案例:**
- 导出带有描述字段的元数据表
- 处理包含 XML 或大型 JSON 文档的表
- 专注于结构化数据，忽略二进制内容
- 针对具有许多大型列的表进行性能优化

### SQL 文件示例

创建文件 `query.sql`:

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

然后执行:

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

## 架构

```
┌─────────────┐
│  SQL        │
│  文件       │
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
│  .vortex 文件            │
│  (columnar binary)       │
└──────────────────────────┘
```

## 运作方式

1. **SQL 读取**: SQL 文件被加载到内存中
2. **启动 SQLcl**: 启动带有 Oracle 连接的进程
3. **配置会话**:
   - `SET SQLFORMAT JSON` 用于 JSON 导出
   - `SET NLS_NUMERIC_CHARACTERS='.,';` 以避免区域设置问题
4. **执行查询**: 通过 stdin 发送 SQL 查询
5. **捕获输出**: 完整读取 JSON stdout
6. **提取 JSON**: 隔离结构 `{"results":[{"items":[...]}]}`
7. **推断模式**: Vortex 模式从第一条记录自动推断
8. **转换记录**: 每个 JSON 对象转换为 Vortex 列
9. **写入文件**: 使用 Tokio 会话创建二进制 Vortex 文件

## 支持的数据类型

JSON 类型到 Vortex 类型的转换自动进行:

| JSON 类型 | Vortex 类型 | Nullable | 注释 |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | 推断为可空字符串 |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (整数) | `Primitive(I64)` | ✅ | 通过 `is_f64() == false` 检测 |
| `number` (浮点数) | `Primitive(F64)` | ✅ | 通过 `is_f64() == true` 检测 |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | 序列化为 JSON 字符串 |
| `object` | `Utf8` | ✅ | 序列化为 JSON 字符串 |

**注意**: 所有类型都是可空的，以处理 Oracle NULL 值。

## 日志和调试

应用程序使用 `tracing` 进行日志记录。消息显示在 stderr 上，带有日志级别。

日志包括:
- Oracle 连接
- 处理的记录数
- 推断的模式
- 错误和警告

## 验证生成的 Vortex 文件

要验证生成的文件，请使用 `vx` 工具:

```bash
# 安装 vx (Vortex CLI 工具)
cargo install vortex-vx

# 浏览 Vortex 文件
vx browse output.vortex

# 显示元数据
vx info output.vortex
```

## 限制和注意事项

- **复杂类型**: 嵌套的 JSON 对象和数组被序列化为字符串
- **内存缓冲**: 记录当前在写入前被缓冲 (未来可能优化)
- **固定模式**: 仅从第一条记录推断 (后续记录必须匹配)
- **安全性**: 密码作为 CLI 参数传递 (使用 `ps` 可见)。在生产环境中使用环境变量。
- **LOB 类型**: 默认情况下，LOB 列 (CLOB, BLOB, NCLOB) 包含在内。使用 `--skip-lobs` 排除它们以获得更好的性能和更小的文件大小。

## 开发

### Debug 模式构建

```bash
cargo build
```

### Release 模式构建

```bash
cargo build --release
```

二进制文件将位于 `target/release/oracle2vortex` (release 模式下约 46 MB)。

### 测试

```bash
cargo test
```

### 手动测试

带有凭据的测试文件位于 `tests_local/` (gitignored):

```bash
# 创建测试查询
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# 执行
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## 许可证

Copyright (c) 2026 William Gacquer

本项目采用 EUPL-1.2 (European Union Public Licence v. 1.2) 许可。

**重要 - 商业使用限制:**  
未经作者事先书面同意，禁止将本软件用于商业用途。  
如需商业许可，请联系: **oracle2vortex@amilto.com**

查看 [LICENSE](LICENSE) 文件了解许可证全文。

## 作者

**William Gacquer**  
联系方式: oracle2vortex@amilto.com

## 测试历史

项目已在 Oracle 生产数据库上验证:

- ✅ **简单测试**: 10 条记录，3 列 → 5.5 KB
- ✅ **复杂测试**: 100 条记录，417 列 → 1.3 MB
- ✅ **验证**: 文件可使用 `vx browse` 读取 (Vortex v0.58)

## 项目结构

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # 此文件
├── IMPLEMENTATION.md       # 技术文档
├── .gitignore             # 排除 tests_local/ 和凭据
├── src/
│   ├── main.rs            # 带 tokio runtime 的入口点
│   ├── cli.rs             # Clap 参数解析
│   ├── sqlcl.rs           # 带 CONNECT 的 SQLcl 进程
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # 转换 JSON→Vortex (API 0.58)
│   └── pipeline.rs        # 完整编排
├── examples/
│   ├── README.md
│   └── sample_query.sql   # 示例查询
└── tests_local/           # 带凭据的测试 (gitignored)
```

## 主要依赖项

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## 资源

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
