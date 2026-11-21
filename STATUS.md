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

## Horizon Client (HUD داخل اللعبة) — الشريحة الأولى
- **يعمل ومُتحقَّق منه بلقطة شاشة على CI:** تشغيل Minecraft 1.8.9 فعلياً عبر `hrc-cli` مع Java Agent، ورسم FPS وCPS وKeystrokes فوق القائمة الرئيسية (`glError=0`).
- الآلية: `-javaagent` يضيف استدعاءً في `org.lwjgl.opengl.Display.update(Z)` (LWJGL 2) — لا mappings ولا تعديل لملفات Minecraft.
- نطاقها الحالي: LWJGL 2 فقط (1.8 – 1.12.2). كل المودات مطفأة افتراضياً وتُضبط عبر `horizon-client.properties` في مجلد اللعبة.
- ملاحظة: في CI يفشل أول تشغيل أحياناً بخطأ `LinuxDisplay.getAvailableDisplayModes` تحت X افتراضي؛ لا يظهر على Windows الحقيقي، والسكربت يعيد المحاولة.
- **لم يُنفَّذ بعد:** قائمة داخل اللعبة (Right Shift) وتعديل أماكن الـ HUD، Toggle Sprint/Zoom/Coordinates/Armor (تحتاج داخل Minecraft)، LWJGL 3 (1.13+)، OpenGL core profile (1.17+)، ودمج الـ agent في اللانشر (خيار تفعيل + شحنه في الـ Installer).
