# الحالة الصادقة (STATUS)

آخر تحديث: 2026-11-21

## كُتب وبُني على CI
| الوحدة | الملف | الحالة |
|---|---|---|
| منصة/مسارات، rules، Version JSON، Launch Core | `hrc-core` | اختبارات وحدة |
| Manifest، Download (SHA1/Range)، Java Runtime، Install | `hrc-core` | تثبيت فعلي مُختبَر (انظر أدناه) |
| Instances + Settings + Offline UUID | `instance.rs`, `settings.rs`, `launch.rs` | اختبارات وحدة |
| واجهة Tauri + TypeScript | `src-tauri/`, `ui/` | تُصرَّف وتُبنى EXE على CI |
| Installer (Inno Setup) | `installer/horizon.iss` | يُبنى على CI |

## اختُبر فعلاً (GitHub Actions)
- `cargo test`: 21 اختباراً ناجحاً على Ubuntu وWindows.
- تثبيت من خوادم Mojang مع SHA1: 1.8.9 و1.12.2 و1.16.5 و1.20.1.
- بناء `horizon-rust-clean.exe` و`hrc-cli.exe` والـ Installer على Windows.
- فحص أنواع TypeScript للواجهة (`tsc`، بلا أخطاء).

## لم يُختبر بعد (لا تعتمد عليه)
- **تشغيل اللعبة فعلياً** من الواجهة أو الـ CLI، والتبديل بين الإصدارات.
- تنزيل Java Runtime وتشغيله على Windows، وأحدث إصدار مستقر، وBeta/Alpha.
- مظهر الواجهة وسلوك الأزرار على جهاز حقيقي (لم تُفتح الواجهة على شاشة).
- تحتاج النافذة WebView2 (موجود عادةً في Windows 10/11 المحدّث).

## لم يُنفَّذ بعد
Microsoft Authentication (حالياً Offline للتجربة فقط)، وضعا Vanilla/Performance (Sodium/Lithium)،
عرض RAM المستخدمة أثناء اللعب، توقيع رقمي للـ Installer.
