# Changelog

## Unreleased

### Added
- **Browse** page (sidebar): search Modrinth for mods, resource packs and shaders and install into a chosen instance. Mods need a loader chosen first (Fabric, Forge, NeoForge or Quilt) and only go to an instance with that loader. Unit-tested: the search URL rules. NOT yet tested: the page in the running app, real downloads of packs.

- Browse can also search CurseForge with the player's OWN API key (Settings; or env HRC_CURSEFORGE_KEY). No key is bundled. Unit-tested: URL, reply parsing, file choice, CDN-only downloads. NOT tested against the real API (the build environment cannot reach api.curseforge.com). Dependencies are not installed; files whose author disabled third-party downloads are refused with a message.

### Changed
- The launcher is named **MClauncher**: window, installer (`MClauncher-Setup.exe`), shortcuts. Its data folder is `MClauncher` in the user's data folder (Windows: `%APPDATA%\MClauncher`); an existing HorizonRustClean data folder is moved there once, never overwriting. The program file is still `horizon-rust-clean.exe`. Migration is unit-tested; the installer and the move on a real Windows profile are not yet tested.
- "Horizon Client (beta)" is now called "Rust" in the UI.
- "Horizon (Rust tools)" and "Horizon Rust (experimental)" no longer appear in the create-instance list or the tabs (existing instances of those kinds still work).

## 0.4.0 — 2026-11-22

### Added
- **Horizon Rust (experimental)**: a small native voxel game written from scratch in Rust (wgpu + winit). It is NOT Minecraft (no Mojang code, worlds, servers, mods or Java). Early prototype: generated terrain, fly camera, FPS/position HUD.
- **Horizon (Rust tools)** environment: original Minecraft (Java), any version, with nothing injected into the game; the launcher adds the optional performance preset and the Rust tools (log cleaner, crash cleaner, screenshot sorter).
- CI: launch tests for Horizon (Rust tools) 1.20.1, install tests for Rust tools 1.20.1 / 26.3, informational launch test for vanilla 26.3.

## 0.3.1 — 2026-11-22

### Added
- **Network check** (Settings → Network check, and `hrc-cli netcheck`): tests every official server the launcher needs (Mojang, Fabric meta, Quilt meta, Forge, NeoForge, Modrinth) and shows the real failure reason.
- Real network errors: the underlying cause (DNS / TLS / refused connection) is shown instead of only "error sending request"; metadata requests retry 3 times; 20 s connect timeout; the OS certificate store is trusted (helps with antivirus HTTPS scanning).
- Mod compatibility check: warns when a mod belongs to another loader or needs another Minecraft version (Fabric/Quilt version ranges; Forge/NeoForge by loader type only).
- NeoForge support for the new Minecraft version numbering (26.x).
- CI: real game launch tests, instance-isolation test, Windows installer install test (see STATUS.md).

### Fixed
- `hrc-cli mod` installed mods outside the folder the launcher uses; it now targets the instance's `game/mods`.

## 0.3.0
- Independent environments: Vanilla, Fabric, Quilt, Forge, NeoForge, Horizon Client (beta). Each instance has its own mods / resource packs / shader packs.
- Modrinth mod manager (search, suggested light Fabric mods, required dependencies, SHA1 check).
- Horizon "Rust mods" (beta): log cleaner, crash cleaner, screenshot sorter (all off by default).
- Windows installer now bundles `horizon-client.jar`.

## 0.2.0
- Tauri GUI (Play / Instances / Downloads / Java / Console / Settings), Horizon Client Java agent (FPS / CPS / Keystrokes on 1.8–1.12.2), profiles and performance presets, Inno Setup installer.

## 0.1.x
- Rust core: version manifest, SHA1 downloads with resume, Mojang Java runtime manager, launch for legacy and modern version JSON, CLI.
