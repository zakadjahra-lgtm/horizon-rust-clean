# الحالة الصادقة (STATUS)

آخر تحديث: 2026-11-21

## كُتب (كود حقيقي، غير مُصرَّف بعد)
| الوحدة | الملف | اختبارات وحدة مكتوبة |
|---|---|---|
| منصة/مسارات | `platform.rs` | — |
| تقييم rules | `rules.rs` | 4 |
| Version JSON (قديم+حديث+inheritsFrom) | `version.rs` | 2 |
| Launch Core (classpath، placeholders، أمر Java، تشغيل) | `launch.rs` | 4 |
| Manifest الرسمي | `manifest.rs` | 1 |
| Download Manager (SHA1، Range، إعادة محاولة) | `download.rs` | 1 |
| Java Runtime Manager (Mojang) | `java.rs` | 2 |
| تثبيت client/libs/natives/assets | `install.rs` | — |
| CLI للاختبار | `hrc-cli` | — |

## لم يُنفَّذ بعد
Tauri + واجهة TypeScript، Instances (SQLite)، Microsoft Auth، Clean/Performance،
تغليف EXE وInstaller.

## ما اختُبر فعلاً (GitHub Actions، تشغيل 44900738335)
- `cargo test`: 14 اختباراً ناجحاً على Ubuntu وWindows.
- تثبيت فعلي من خوادم Mojang: 1.8.9 (768 ملفاً)، 1.12.2 (1342)، 1.16.5 (2665)، 1.20.1 (3653) — كلها ناجحة مع التحقق من SHA1.
- بناء `hrc-cli.exe` لـ Windows (artifact).

## ما لم يُختبر بعد (لا تعتمد عليه)
- **تشغيل اللعبة فعلياً** (سيرفر CI بلا شاشة) والتبديل بين الإصدارات.
- تنزيل Java Runtime وتشغيله، فك natives على Windows، الإصدارات القديمة (Beta/Alpha)، أحدث إصدار مستقر.
- اختبار `launch` يحتاج تشغيلاً محلياً على جهازك.

## الخطوة التالية
```
cargo test -p hrc-core                  # اختبارات الوحدة
cargo run -p hrc-cli -- install 1.8.9
cargo run -p hrc-cli -- launch 1.8.9 Steve 2048   # يحتاج شاشة (display)
```
