# MClauncher — STATUS (v1.1.0)

> Rule of this file: nothing is marked as working unless a test actually ran it. "Verified" means a CI job did it on the stated platform; everything else is listed under "Not verified".

## Verified by CI for v0.3.1 (Linux x64, Xvfb + Mesa software GL; Windows x64 runner)

| Check | Result |
|---|---|
| **Real launch** (Minecraft window appears, process still alive 45 s later, screenshot has real content, no crash marker, Java from launcher folder) | Vanilla 1.8.9, 1.12.2, 1.16.5, 1.20.1, 1.21.1 · Fabric 1.20.1, 1.21.1 · Quilt 1.20.1 · Forge 1.12.2, 1.16.5, 1.20.1 · NeoForge 1.21.1 |
| I personally looked at screenshots of | Forge 1.20.1 and NeoForge 1.21.1 (Minecraft welcome screen). The other launches passed the automatic checks only. |
| **Java Runtime automatic** | In every launch the Mojang runtime was downloaded into the launcher's own data folder (`runtimes/java-runtime-*`/`jre-legacy`); no system Java was used. |
| **Mods really load** | Sodium 0.5.13 and Lithium 0.11.4, downloaded from Modrinth, were loaded by Fabric 1.20.1 (seen in the game log). |
| **Instance isolation** | Vanilla + Fabric + Quilt on the same Minecraft version in one data folder: separate folders, separate mods (Fabric has Lithium only, Quilt has Ferrite Core only, Vanilla has no mods folder), separate natives, no shared jars. |
| **Vanilla 26.3 real launch (Linux)** | Passed once in CI after adding Vulkan (lavapipe) to the job: window appeared, process alive after load, screenshot with real content (29,906 colors); I looked at the screenshot: the Minecraft Java Edition welcome screen rendered. One run only; the job stays informational until it passes repeatedly. Not verified on Windows. |
| **Windows installer (x64)** | Silent install; `horizon-rust-clean.exe`, `hrc-cli.exe`, `horizon-client.jar` present; `hrc-cli.exe` ran on Windows (netcheck reached Mojang, Fabric, Quilt, Forge, NeoForge; Fabric 1.20.1 install completed); the GUI stayed alive 20 s with window title "Horizon Rust Clean". |
| Install-only matrix | Fabric 1.16.5/1.18/1.20.1/1.21.1/26.3 · Quilt 1.18.2/1.20.1/1.21.1 · Forge 1.12.2/1.16.5/1.18.2/1.20.1/1.21.1/26.3 · NeoForge 1.20.4/1.21.1/26.3 |

## Horizon (Rust tools) — plain Minecraft + launcher-side Rust tools (new in 0.4.0)

Environment id `rusttools`: runs original Minecraft (Java) with no mods and no agent. In the GUI Play path the launcher applies the chosen performance preset and the enabled Rust tools (log cleaner, crash cleaner, screenshot sorter) before launching.
**Verified by CI:** install for 1.20.1 and 26.3; real launch of 1.20.1 (window, alive after load, screenshot, no crash) — note the CLI launch path used by CI is plain vanilla; the Tauri path that adds the preset/tools was not launched by CI (preset flags were accepted by the JVM in an earlier test, and the Rust tools are unit-tested).
**Not verified:** FPS effect of the preset; the GUI flow.

**Minecraft 26.3 launch:** the CI run downloaded the right Java runtime (java-runtime-epsilon) and started the game, but the game could not create a renderer under the CI virtual display ("Couldn't find matching GLX visual", Vulkan unsupported). So 26.x launching is **not verified**; install is.

## Horizon Rust (experimental) — native Rust game (new in 0.4.0)

**What it is:** a small voxel game prototype written from scratch in Rust (`crates/hrc-rustgame`: wgpu + winit, no Mojang code or assets). Generated terrain, face-culled chunk meshes, fly camera (WASD / Space / Shift / Ctrl, mouse look), bitmap-font HUD (FPS, position, chunks, faces), position saved in the instance folder, `horizon-rust.properties` for seed/radius/vsync. In the launcher it is the environment "Horizon Rust (experimental)": no install download, no Java.

**It is NOT Minecraft:** it cannot open Minecraft worlds, join Minecraft servers, or run Java mods/resource packs/shaders. It has no textures, no physics or collisions, no block placing/breaking, no sound, no entities.

**Verified by CI:**
- Unit tests (terrain determinism, mesh face culling incl. across chunk borders, font, config/args/save parsing) pass on Linux and Windows.
- Real GPU-API render without a window on Linux (Vulkan software adapter llvmpipe): a frame was rendered and saved as PNG; I looked at the screenshot (terrain + HUD correct). 19 fps at 1280x720 on that CPU renderer, 145k faces.
- Window mode under Xvfb on Linux stayed alive 20 s and drew frames (screenshot viewed; 42 fps on CPU rendering).
- Windows x64 (installed by the installer): the exe is present and rendered a frame on the "Microsoft Basic Render Driver" (DX12, software) adapter.

**NOT verified:** performance on a real GPU (the 591 fps seen in your screenshot was not produced by anything I built); the launcher flow (create instance -> Install -> Play spawning the game from the GUI) has not been clicked through by any test; interactive input (mouse capture, keys) was never exercised; macOS.

## NOT verified (do not assume these work)
- **Launching the game on Windows.** The Windows CI runner has no GPU, so only install + GUI start were tested there. Launch was tested on Linux with software rendering.
- Real-GPU performance, and any **FPS gain** (presets and Sodium/Lithium are not benchmarked).
- The GUI beyond starting: clicking through Create / Install / Play / Content / Get mods / Rust mods / Network check has not been exercised by an automated or human test.
- Launch of: Forge < 1.12.2, Minecraft 1.13–1.15, Fabric/Quilt/Forge/NeoForge 26.x, Quilt 1.21.1, NeoForge 1.20.4 (these were install-tested only or not tested).
- **The `meta.fabricmc.net` error reported on the author's PC.** It could not be reproduced: the CI Linux and Windows runners reach it fine. The cause is probably local (firewall, antivirus HTTPS scanning, VPN, DNS, ISP block). v0.3.1 adds a Network check and clearer errors; whether that fixes the reporter's PC is **unconfirmed**.
- Horizon Client HUD (FPS/CPS/Keystrokes) on Minecraft 1.13+ (not implemented; it works on 1.8–1.12.2 only). Rust mods inside a real launch (only unit-tested).
- Microsoft sign-in: intentionally not implemented. Offline accounts only (cannot join online-mode servers).
- The installer is **not code-signed**; Windows SmartScreen will warn.

---

## Earlier notes (history)

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

## Loaders: what is actually tested (CI, install only)

`hrc-cli install` (official Fabric/Quilt meta APIs and official Forge/NeoForge installers) succeeded for:
Fabric 1.16.5, 1.18, 1.20.1, 1.21.1, 26.3 · Quilt 1.18.2, 1.20.1, 1.21.1 · Forge 1.12.2, 1.16.5, 1.18.2, 1.20.1, 1.21.1, 26.3 · NeoForge 1.20.4, 1.21.1, 26.3.

This is **install only**: the profile was created and the version JSON exists. Launching these loaders in a real game is NOT verified, so "supports all versions" is not claimed. Quilt 26.3 and Forge below 1.12.2 are untested.

Mod compatibility check (unit-tested): detects Fabric/Quilt/Forge/NeoForge mods from the jar metadata, warns when a mod's loader doesn't match the instance, and checks the Minecraft version range for Fabric/Quilt mods. Forge/NeoForge mods are checked by loader type only (version ranges in mods.toml are not parsed). Network errors now show the real cause and are retried 3 times; the OS certificate store is trusted (helps with antivirus HTTPS scanning).
