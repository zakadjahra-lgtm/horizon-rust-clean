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

## المراجع والتراخيص + Profiles + FPS presets (2026-11-22)
- **وثيقة المراجع:** `docs/REFERENCES.md` (فحص المرفقات وصفحات المستودعات)، و`THIRD_PARTY_NOTICES.md` (ASM مشحون داخل الـ agent).
- **اختُبر على CI:** 35 اختبار وحدة (Profiles، presets، أهلية Sodium/Lithium حسب الإصدار، ربط profile بكل instance).
- **اختُبر على CI (تشغيل حقيقي):** 1.8.9 اشتغلت بإعدادات JVM الخاصة بـ `balanced` مع الـ agent، والـ JVM قبل الأعلام (لا `Unrecognized VM option`) على Java 8 فقط.
- **لم يُقَس:** أي مكسب فعلي في FPS من الـ presets (لا benchmark بعد). لا يجوز ذكر أرقام تحسين قبل القياس.
- **لم يُختبر:** الـ presets على Java 17/21 (1.17+)، لأن الـ agent وطبقة HUD تدعم 1.8–1.12.2 فقط حالياً.
- **غير مربوط بعد بالواجهة:** Profiles موجودة في `hrc-core` فقط؛ لا شاشة لها في اللانشر ولا أوامر Tauri، والـ Installer لا يشحن `horizon-client.jar` بعد.
- **غير مبني:** Mod Manager (Modrinth + Fabric)، وقائمة داخل اللعبة، ودعم LWJGL 3.

## Independent environments (Vanilla / Fabric / Quilt / Forge / NeoForge / Horizon Client)

Each instance has its own folder with its own `mods/`, `resourcepacks/`, `shaderpacks/`, worlds and settings. Nothing is shared between environments except the downloaded game files (libraries/assets).

**Verified on CI (commit 92e99dd):** unit tests (folder isolation, mod toggle/list/delete, path-safety); `hrc-cli install` succeeds for Fabric 1.20.1, Quilt 1.20.1, Forge 1.20.1 and 1.12.2, NeoForge 1.21.1, Horizon 1.8.9 and 1.20.1 (the launch profile is created and the version JSON exists); the Tauri app and the TypeScript UI compile; the installer now bundles `horizon-client.jar`.

**NOT verified yet:** actually launching a Fabric/Quilt/Forge/NeoForge game; the new UI screens (Content page, environment tabs, Horizon panel) on a real machine; FPS gains of the performance presets (not benchmarked). The Horizon "Rust mods" and the curated Fabric light-mod downloader (Modrinth) are not built yet — mods are currently added by dropping files into the instance's folder via "Open folder".

## Mod manager and Rust mods (beta)

**Verified on CI:** `hrc-cli mod` downloads real mods from the Modrinth API with Modrinth's SHA1 check and required dependencies: Sodium and Lithium on Fabric 1.20.1, Lithium on Quilt 1.20.1, Ferrite Core on Horizon 1.20.1 (Fabric base). Unit tests cover version/file picking, URL encoding and the Rust mods (log cleaner, crash cleaner, screenshot sorter; all off by default, `latest.log` is never deleted).

**Not verified:** that the downloaded mods actually load and run in a launched game; the "Get mods" UI and Rust-mod toggles on a real machine; FPS gains. Sodium's license is PolyForm Shield, so it is downloaded from Modrinth on request and never bundled.

**Honest scope:** "Rust mods" are small launcher-side tools that run before the game starts; they do not modify Minecraft. The in-game HUD (FPS/CPS/Keystrokes) still works only on 1.8-1.12.2; on 1.13+ Horizon runs on a Fabric base without a HUD yet (LWJGL3 needs a different hook). Accounts: offline only (no Microsoft login, by decision).
