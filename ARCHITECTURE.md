# Horizon Rust Clean — Architecture

## 1. كيف يعمل Rust مع Java

```
┌─────────────── Tauri (نافذة النظام + WebView) ───────────────┐
│  TypeScript UI  ──invoke()/events──►  src-tauri (أوامر رقيقة) │
└───────────────────────────────┬──────────────────────────────┘
                                │ استدعاء مباشر (نفس العملية)
                      ┌─────────▼──────────┐
                      │  hrc-core (Rust)   │  المحرك الحقيقي
                      │  manifest · version│
                      │  download · java   │
                      │  launch · instance │
                      │  auth · clean      │
                      └─────────┬──────────┘
                                │ std::process::Command
                      ┌─────────▼──────────┐
                      │ java -cp ... Main  │  Minecraft الأصلية فقط
                      └────────────────────┘
```

- Rust لا يحتوي أي منطق للعبة. وظيفته: جلب ملفات Mojang الرسمية، التحقق منها،
  اختيار Java، بناء سطر الأوامر، تشغيل عملية Java كعملية ابن، وقراءة stdout/stderr.
- Java تُشغَّل كعملية منفصلة. عند إغلاق اللعبة يعود اللانشر لحالته الخاملة (لا خدمات خلفية).
- `hrc-core` مكتبة مستقلة عن Tauri، لذلك يمكن اختبارها عبر `cargo test` وعبر CLI (`hrc-cli`)
  دون واجهة.

## 2. دعم الإصدارات القديمة والحديثة

| الجانب | قديم (≤1.12.2) | حديث (≥1.13) | كيف نتعامل معه |
|---|---|---|---|
| Arguments | `minecraftArguments` نص واحد | `arguments.game` / `arguments.jvm` مع `rules` | `VersionJson` يقبل الشكلين، ويحوّلهما لقائمة موحدة |
| JVM args | غير موجودة (تُضاف يدوياً: natives, classpath) | موجودة في JSON | إن غابت نضيف الافتراضية القياسية |
| Natives | مكتبات `natives` + `extract` | مكتبات بـ classifier أو اسم `natives-*` | فك الضغط إلى `natives/` الخاص بالـ instance مع احترام `exclude` |
| Assets | `assets/virtual/legacy` أو `resources` (≤1.7.2) | `assets/objects` | `assets_index.map_to_resources/virtual` |
| Java | 8 | 8 / 16 / 17 / 21 | `javaVersion.majorVersion` في JSON، وإلا الافتراضي 8 |
| Inheritance | — | `inheritsFrom` (Fabric/Forge) | دمج JSON الأب مع الابن |
| Alpha/Beta | JSON بدون assetIndex أحياناً | — | معالجة الحقول الاختيارية |

## 3. الوحدات (crate `hrc-core`)

| الملف | المسؤولية |
|---|---|
| `platform.rs` | OS/arch ومسارات التطبيق |
| `manifest.rs` | تنزيل `version_manifest_v2.json` وتصفية الأنواع |
| `version.rs` | نماذج Version JSON + دمج `inheritsFrom` |
| `rules.rs` | تقييم `rules` (os, features) |
| `download.rs` | تحميل متوازي، SHA1، استكمال Range، إعادة محاولة |
| `install.rs` | تثبيت client + libraries + assets + natives |
| `java.rs` | اكتشاف Java وتنزيل Runtime الرسمي من Mojang |
| `launch.rs` | بناء الأمر وتشغيل العملية |
| `instance.rs` | Instances (مجلد مستقل + JSON إعدادات) |
| `auth.rs` | Microsoft/Xbox/Minecraft device-code flow |
| `clean.rs` | وضعا Vanilla / Performance (Modrinth API) |

## 4. بنية المجلدات على القرص

```
<data>/
  shared/versions/<id>/<id>.json, <id>.jar   # ملفات Mojang للقراءة فقط
  shared/libraries/…   shared/assets/{indexes,objects}/…
  runtimes/<component>/…                      # Java متعددة
  instances/<uuid>/instance.json
  instances/<uuid>/game/ (saves, options.txt…) + natives/
  accounts.json   (الرموز في مخزن النظام keyring)
```
الملفات المشتركة تُحمَّل مرة واحدة وتُتحقق بـ SHA1؛ كل instance يملك مجلد لعبة ونatives خاصة،
فلا اختلاط بين الإصدارات.

## 5. الوضعان
- **Vanilla Clean:** لا مودات ولا Java agents ولا وسائط إضافية سوى ما في JSON + RAM.
- **Performance Clean:** يضيف Fabric loader + Sodium/Lithium من Modrinth عندما يدعم الإصدار ذلك،
  داخل مجلد instance منفصل حتى لا يلوّث Vanilla.

## 6. الحالة الصادقة
انظر `STATUS.md` لما كُتب وما اختُبر فعلاً.
