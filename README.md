<p align="center"><img src="assets/logo.png" width="120" alt="logo"></p>

# Horizon Rust Clean

لانشر Minecraft Java Edition مبني بـ **Rust**، يشغّل ملفات Minecraft الأصلية الرسمية عبر **Java** ويدير كل ما حولها (الإصدارات، التحميل، Java، التشغيل).
لا يوجد محرك بديل ولا إعادة برمجة للعبة.

> ⚠️ **المشروع قيد التطوير (نسخة تجريبية).** توجد الآن واجهة رسومية (Tauri) تعمل فوق المحرك، لكن تشغيل اللعبة منها لم يُختبر بعد. تسجيل Microsoft وPerformance Clean غير منفذين.

## الحالة الحالية

| الميزة | الحالة |
|---|---|
| قراءة Version Manifest الرسمي من Mojang | ✅ مكتوبة |
| قراءة Version JSON (الصيغة القديمة والحديثة + `inheritsFrom`) | ✅ مكتوبة + اختبارات وحدة |
| تقييم `rules` وبناء classpath وأمر التشغيل | ✅ مكتوبة + اختبارات وحدة |
| تحميل Client / Libraries / Assets / Natives مع SHA1 واستكمال الملفات | ✅ **مُختبَرة تثبيتاً** على 1.8.9 و1.12.2 و1.16.5 و1.20.1 |
| تحميل Java Runtime الرسمي تلقائياً | ⚠️ مكتوبة، لم تُختبر بعد |
| **تشغيل اللعبة فعلياً** | ⚠️ مكتوبة، **لم تُختبر بعد** |
| Instances (إنشاء/حذف/نسخ/إعادة تسمية/RAM لكل instance، ملفات JSON) | ✅ مكتوبة + اختبارات وحدة |
| واجهة Tauri (Dark): Play / Instances / Downloads / Java / Console / Settings | ✅ تُبنى على CI — ⚠️ لم تُجرَّب يدوياً بعد |
| Microsoft Authentication | ⏳ لم تُنفَّذ |
| وضعا Vanilla Clean / Performance Clean | ⏳ لم تُنفَّذ |
| Installer احترافي | ⏳ لم يُنفَّذ |

التفاصيل الدقيقة لما اختُبر وما لم يُختبر في [`STATUS.md`](STATUS.md)، والتصميم في [`ARCHITECTURE.md`](ARCHITECTURE.md).

## تجربة أداة سطر الأوامر (Windows)

حمّل `hrc-cli.exe` من صفحة [Releases](../../releases)، ثم من PowerShell:

```powershell
.\hrc-cli.exe list                      # الإصدارات الرسمية
.\hrc-cli.exe install 1.8.9             # تثبيت إصدار
.\hrc-cli.exe launch 1.8.9 Steve 2048   # تشغيل (Offline للتجربة، 2048MB RAM)
```

ملفات اللعبة تُحفظ في مجلد بيانات التطبيق، ويمكن تغييره بمتغير البيئة `HRC_DATA`.
إن فشل `launch` أرسل المخرجات كاملة لأنها أهم اختبار متبقٍّ.

## البنية

```
crates/hrc-core   المحرك: manifest · version · rules · download · java · install · launch
crates/hrc-cli    أداة سطر أوامر للاختبار بدون واجهة
.github/workflows اختبارات CI على Linux وWindows وتثبيت فعلي لأربعة إصدارات
```

## البناء

```
cargo test --workspace
cargo build --release -p hrc-cli
```
