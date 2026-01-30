# oracle2vortex

एक CLI एप्लिकेशन जो Oracle तालिकाओं को SQLcl के माध्यम से JSON स्ट्रीमिंग के साथ Vortex प्रारूप में निर्यात करता है।

## विवरण

`oracle2vortex` निम्नलिखित का उपयोग करके Oracle डेटा निर्यात की अनुमति देता है:
- **SQLcl** कनेक्शन और मूल JSON निर्यात के लिए
- **स्ट्रीमिंग** निर्यात समाप्त होने की प्रतीक्षा किए बिना डेटा को तुरंत संसाधित करने के लिए
- **स्वचालित रूपांतरण** स्कीमा अनुमान के साथ स्तंभीय Vortex प्रारूप में

✅ **परियोजना पूर्ण और उत्पादन में परीक्षणित** - वास्तविक डेटाबेस पर 417 कॉलम की तालिका के साथ सत्यापित।

## पूर्वापेक्षाएँ

- **Rust nightly** (Vortex crates द्वारा आवश्यक)
- **SQLcl** स्थापित (या `--sqlcl-path` के साथ पथ निर्दिष्ट करें)
- सुलभ Oracle डेटाबेस

### Rust nightly स्थापना

```bash
rustup install nightly
cd oracle2vortex
rustup override set nightly
```

### SQLcl स्थापना

SQLcl यहाँ से डाउनलोड करें: https://www.oracle.com/database/sqldeveloper/technologies/sqlcl/

या Linux पर:
```bash
# /opt/oracle/sqlcl/ में स्थापना का उदाहरण
wget https://download.oracle.com/otn_software/java/sqldeveloper/sqlcl-latest.zip
unzip sqlcl-latest.zip -d /opt/oracle/
```

## स्थापना

```bash
git clone <repo-url>
cd oracle2vortex
cargo build --release
```

निष्पादन योग्य फ़ाइल `target/release/oracle2vortex` में उपलब्ध होगी।

## उपयोग

### मूल सिंटैक्स

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

### विकल्प

| विकल्प | संक्षिप्त | विवरण | डिफ़ॉल्ट |
|--------|--------|-------------|--------|
| `--sql-file` | `-f` | क्वेरी युक्त SQL फ़ाइल का पथ | (आवश्यक) |
| `--output` | `-o` | आउटपुट Vortex फ़ाइल का पथ | (आवश्यक) |
| `--host` | | Oracle होस्ट | (आवश्यक) |
| `--port` | | Oracle पोर्ट | 1521 |
| `--user` | `-u` | Oracle उपयोगकर्ता | (आवश्यक) |
| `--password` | `-p` | Oracle पासवर्ड | (आवश्यक) |
| `--sid` | | Oracle SID या सेवा नाम | (आवश्यक) |
| `--sqlcl-path` | | SQLcl निष्पादन योग्य का पथ | `sql` |
| `--auto-batch-rows` | | प्रति बैच पंक्तियों की संख्या (0 = अक्षम) | 0 |
| `--skip-lobs` | | Oracle LOB प्रकारों को छोड़ें (CLOB, BLOB, NCLOB) | false |

### स्वचालित बैचिंग (बड़ी तालिकाएँ)

लाखों या अरबों पंक्तियों वाली तालिकाओं को स्थिर मेमोरी उपयोग के साथ संसाधित करने के लिए, `--auto-batch-rows` विकल्प का उपयोग करें:

```bash
# 50000 पंक्तियों के बैचों में प्रसंस्करण
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

**यह कैसे काम करता है:**
1. स्वचालित रूप से आपकी क्वेरी को `OFFSET/FETCH` के साथ लपेटता है
2. SQLcl को कई बार निष्पादित करता है (प्रति बैच एक बार)
3. मेमोरी में सभी परिणामों को संचित करता है
4. सभी डेटा युक्त एकल Vortex फ़ाइल लिखता है

**सीमाएँ:**
- Oracle 12c+ की आवश्यकता है (OFFSET/FETCH सिंटैक्स)
- आपकी क्वेरी में पहले से OFFSET/FETCH या ROWNUM नहीं होना चाहिए
- अनुशंसित: संगत क्रम के लिए ORDER BY जोड़ें

**मेमोरी:** स्वचालित बैचिंग के साथ, उपयोग की गई मेमोरी = बैच आकार × 2 (JSON + Vortex)  
उदाहरण: 50000 पंक्तियाँ × 1 KB = प्रति बैच 100 MB (संपूर्ण तालिका लोड करने के बजाय)

**यह भी देखें:** अधिक विवरण के लिए `BATCH_PROCESSING.md` और `README_LARGE_DATASETS.md`।

### LOB कॉलम छोड़ना

Oracle LOB प्रकार (CLOB, BLOB, NCLOB) बहुत बड़े हो सकते हैं और विश्लेषण के लिए आवश्यक नहीं हो सकते हैं। उन्हें बाहर करने के लिए `--skip-lobs` का उपयोग करें:

```bash
# फ़ाइल आकार कम करने और प्रदर्शन में सुधार के लिए LOB कॉलम छोड़ें
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

**यह कैसे काम करता है:**
- स्वचालित रूप से LOB डेटा युक्त कॉलमों का पता लगाता है और फ़िल्टर करता है
- LOB को आकार (> 4000 वर्ण) या बाइनरी संकेतकों द्वारा पहचाना जाता है
- पहला लॉग रिकॉर्ड दिखाएगा कि कितने कॉलम छोड़े गए
- बड़े टेक्स्ट/बाइनरी फ़ील्ड वाली तालिकाओं के लिए फ़ाइल आकार और मेमोरी उपयोग को काफी कम करता है

**उपयोग के मामले:**
- विवरण फ़ील्ड वाली मेटाडेटा तालिकाओं का निर्यात
- XML या बड़े JSON दस्तावेज़ों वाली तालिकाओं के साथ काम करना
- बाइनरी सामग्री को नज़रअंदाज़ करते हुए संरचित डेटा पर ध्यान केंद्रित करना
- कई बड़े कॉलम वाली तालिकाओं के लिए प्रदर्शन अनुकूलन

### SQL फ़ाइल के साथ उदाहरण

फ़ाइल `query.sql` बनाएं:

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

फिर निष्पादित करें:

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

## आर्किटेक्चर

```
┌─────────────┐
│  SQL        │
│  फ़ाइल      │
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
│  .vortex फ़ाइल           │
│  (columnar binary)       │
└──────────────────────────┘
```

## संचालन

1. **SQL पढ़ना**: SQL फ़ाइल मेमोरी में लोड होती है
2. **SQLcl प्रारंभ करना**: Oracle कनेक्शन के साथ प्रक्रिया प्रारंभ करना
3. **सत्र कॉन्फ़िगरेशन**:
   - `SET SQLFORMAT JSON` JSON निर्यात के लिए
   - `SET NLS_NUMERIC_CHARACTERS='.,';` लोकेल समस्याओं से बचने के लिए
4. **क्वेरी निष्पादन**: SQL क्वेरी stdin के माध्यम से भेजी जाती है
5. **आउटपुट कैप्चर करना**: JSON stdout की पूर्ण रीडिंग
6. **JSON निष्कर्षण**: संरचना `{"results":[{"items":[...]}]}` को पृथक करना
7. **स्कीमा अनुमान**: Vortex स्कीमा स्वचालित रूप से पहले रिकॉर्ड से अनुमानित होता है
8. **रिकॉर्ड रूपांतरण**: प्रत्येक JSON ऑब्जेक्ट को Vortex कॉलमों में परिवर्तित किया जाता है
9. **फ़ाइल लेखन**: Tokio सत्र के साथ बाइनरी Vortex फ़ाइल बनाई जाती है

## समर्थित डेटा प्रकार

JSON प्रकारों का Vortex प्रकारों में रूपांतरण स्वचालित रूप से होता है:

| JSON प्रकार | Vortex प्रकार | Nullable | नोट्स |
|-----------|-------------|----------|-------|
| `null` | `Utf8` | ✅ | nullable स्ट्रिंग के रूप में अनुमानित |
| `boolean` | `Bool` | ✅ | Via BoolArray |
| `number` (पूर्णांक) | `Primitive(I64)` | ✅ | `is_f64() == false` से पता लगाया गया |
| `number` (फ्लोट) | `Primitive(F64)` | ✅ | `is_f64() == true` से पता लगाया गया |
| `string` | `Utf8` | ✅ | Via VarBinArray |
| `array` | `Utf8` | ✅ | JSON स्ट्रिंग के रूप में क्रमबद्ध |
| `object` | `Utf8` | ✅ | JSON स्ट्रिंग के रूप में क्रमबद्ध |

**नोट**: सभी प्रकार Oracle NULL मानों को संभालने के लिए nullable हैं।

## लॉग और डिबगिंग

एप्लिकेशन लॉग के लिए `tracing` का उपयोग करता है। संदेश लॉग स्तर के साथ stderr पर प्रदर्शित होते हैं।

लॉग में शामिल हैं:
- Oracle से कनेक्शन
- संसाधित रिकॉर्ड की संख्या
- अनुमानित स्कीमा
- त्रुटियाँ और चेतावनियाँ

## उत्पन्न Vortex फ़ाइलों का सत्यापन

उत्पन्न फ़ाइलों को सत्यापित करने के लिए, `vx` टूल का उपयोग करें:

```bash
# vx स्थापित करें (Vortex CLI टूल)
cargo install vortex-vx

# Vortex फ़ाइल ब्राउज़ करें
vx browse output.vortex

# मेटाडेटा प्रदर्शित करें
vx info output.vortex
```

## सीमाएँ और विचार

- **जटिल प्रकार**: नेस्टेड JSON ऑब्जेक्ट और एरे स्ट्रिंग में क्रमबद्ध होते हैं
- **मेमोरी में बफर**: रिकॉर्ड वर्तमान में लिखने से पहले बफर किए जाते हैं (भविष्य में अनुकूलन संभव)
- **निश्चित स्कीमा**: केवल पहले रिकॉर्ड से अनुमानित (बाद के रिकॉर्ड को मेल खाना चाहिए)
- **सुरक्षा**: पासवर्ड CLI तर्क के रूप में पास किया जाता है (`ps` के साथ दृश्यमान)। उत्पादन में पर्यावरण चर का उपयोग करें।
- **LOB प्रकार**: डिफ़ॉल्ट रूप से, LOB कॉलम (CLOB, BLOB, NCLOB) शामिल हैं। बेहतर प्रदर्शन और छोटे फ़ाइल आकार के लिए उन्हें बाहर करने के लिए `--skip-lobs` का उपयोग करें।

## विकास

### Debug मोड में बिल्ड

```bash
cargo build
```

### Release मोड में बिल्ड

```bash
cargo build --release
```

बाइनरी `target/release/oracle2vortex` में होगी (release में ~46 MB)।

### परीक्षण

```bash
cargo test
```

### मैनुअल परीक्षण

क्रेडेंशियल्स वाली परीक्षण फ़ाइलें `tests_local/` में हैं (gitignored):

```bash
# परीक्षण क्वेरी बनाएं
echo "SELECT * FROM my_table WHERE ROWNUM <= 10;" > tests_local/test.sql

# निष्पादित करें
./target/release/oracle2vortex \
  -f tests_local/test.sql \
  -o tests_local/output.vortex \
  --host myhost \
  --port 1521 \
  -u myuser \
  -p mypass \
  --sid MYSID
```

## लाइसेंस

Copyright (c) 2026 William Gacquer

यह परियोजना EUPL-1.2 (European Union Public Licence v. 1.2) के तहत लाइसेंस प्राप्त है।

**महत्वपूर्ण - व्यावसायिक उपयोग प्रतिबंध:**  
लेखक की पूर्व लिखित सहमति के बिना इस सॉफ़्टवेयर का व्यावसायिक उपयोग निषिद्ध है।  
किसी भी व्यावसायिक लाइसेंस अनुरोध के लिए, संपर्क करें: **oracle2vortex@amilto.com**

लाइसेंस का पूर्ण पाठ देखने के लिए [LICENSE](LICENSE) फ़ाइल देखें।

## लेखक

**William Gacquer**  
संपर्क: oracle2vortex@amilto.com

## परीक्षण इतिहास

परियोजना Oracle उत्पादन डेटाबेस पर सत्यापित की गई:

- ✅ **सरल परीक्षण**: 10 रिकॉर्ड, 3 कॉलम → 5.5 KB
- ✅ **जटिल परीक्षण**: 100 रिकॉर्ड, 417 कॉलम → 1.3 MB
- ✅ **सत्यापन**: फ़ाइलें `vx browse` के साथ पठनीय (Vortex v0.58)

## परियोजना संरचना

```
oracle2vortex/
├── Cargo.toml              # 11 dependencies (vortex 0.58, tokio, clap, etc.)
├── README.md               # यह फ़ाइल
├── IMPLEMENTATION.md       # तकनीकी दस्तावेज़ीकरण
├── .gitignore             # tests_local/ और क्रेडेंशियल को बाहर करता है
├── src/
│   ├── main.rs            # tokio runtime के साथ एंट्री पॉइंट
│   ├── cli.rs             # Clap तर्क पार्सिंग
│   ├── sqlcl.rs           # CONNECT के साथ SQLcl प्रक्रिया
│   ├── json_stream.rs     # Parser {"results":[...]}
│   ├── vortex_writer.rs   # रूपांतरण JSON→Vortex (API 0.58)
│   └── pipeline.rs        # पूर्ण ऑर्केस्ट्रेशन
├── examples/
│   ├── README.md
│   └── sample_query.sql   # नमूना क्वेरी
└── tests_local/           # क्रेडेंशियल के साथ परीक्षण (gitignored)
```

## मुख्य निर्भरताएँ

- **vortex-array, vortex-dtype, vortex-buffer, vortex-file, vortex-session, vortex-io** v0.58
- **tokio** v1.40 (async runtime)
- **clap** v4.5 (CLI parsing)
- **serde_json** v1.0 (JSON parsing)
- **anyhow** v1.0 (error handling)

## संसाधन

- [SQLcl Documentation](https://docs.oracle.com/en/database/oracle/sql-developer-command-line/)
- [Vortex Format](https://github.com/spiraldb/vortex)
- [Vortex Crates Documentation](https://docs.rs/vortex-array/)
- [Apache Arrow](https://arrow.apache.org/)
