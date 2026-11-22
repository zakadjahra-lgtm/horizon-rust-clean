# Changelog

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
