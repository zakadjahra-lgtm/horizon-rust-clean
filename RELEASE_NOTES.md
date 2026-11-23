MClauncher v1.0.0 (test build / نسخة تجريبية)

Install: download **MClauncher-Setup.exe** (Windows x64). Not code-signed, so SmartScreen may warn. Data folder: `%APPDATA%\MClauncher` (an older HorizonRustClean folder is moved there once).

New since v0.4.0:
- Renamed to MClauncher.
- **Browse** page: search Modrinth (and CurseForge with your own API key in Settings) for mods, resource packs and shaders; mods require choosing Fabric/Forge/NeoForge/Quilt first and install only into a matching instance.
- "Horizon Client (beta)" is now "Rust"; "Horizon (Rust tools)" and "Horizon Rust (experimental)" are hidden from the new-instance list (old instances still work).

Verified by CI (STATUS.md): launches/installs of Vanilla, Fabric, Quilt, Forge, NeoForge on Linux, instance isolation, Windows installer + GUI start, unit tests on Linux and Windows.
NOT tested: the Browse page in the running app, real downloads of resource packs/shaders, CurseForge against the real API, the installer on a real PC and the data-folder move, launching Minecraft on Windows, real-GPU performance, Minecraft 26.x launch.
Accounts: offline only.
