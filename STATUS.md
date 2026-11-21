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

## ما لم يُختبر (لا تعتمد عليه)
**لا شيء من الكود أعلاه صُرِّف أو شُغِّل بعد.** بيئة هذه الجلسة لا تسمح بتنزيل crates
(الشبكة معطّلة للحزم)، فلم يتم تشغيل `cargo build` ولا `cargo test`. وبالتالي لم يُختبر
تثبيت 1.8.9 / 1.12.2 / 1.16.5 / 1.20.1 / الأحدث ولا تشغيل اللعبة ولا التبديل بين الإصدارات.
المتوقع أن تظهر أخطاء تصريف صغيرة في أول بناء.

## الخطوة التالية
```
cargo test -p hrc-core                  # اختبارات الوحدة
cargo run -p hrc-cli -- install 1.8.9
cargo run -p hrc-cli -- launch 1.8.9 Steve 2048   # يحتاج شاشة (display)
```
