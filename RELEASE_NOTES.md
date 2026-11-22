Horizon Rust Clean v0.4.0 (test build / نسخة تجريبية)

Install: download **HorizonRustClean-Setup.exe** (Windows x64). Not code-signed, so SmartScreen may warn.

New in this build — two Rust-flavoured environments in "New instance":
- **Horizon Rust (experimental)**: a tiny native voxel game written in Rust. NOT Minecraft: no Java, no Minecraft worlds/servers/mods. Prototype (fly around generated terrain with an FPS HUD).
- **Horizon (Rust tools)**: normal Minecraft (Java), any version, nothing injected; launcher-side performance preset + log/crash cleaners + screenshot sorter, all optional.

Tested in CI (details in STATUS.md): everything from v0.3.1 (real launches of Vanilla/Fabric/Quilt/Forge/NeoForge on 1.8.9-1.21.1 on Linux, isolation, Windows installer + GUI start), plus the Rust game rendering a frame on Linux and Windows (software GPU) and Horizon (Rust tools) 1.20.1 launching.

NOT tested: launching Minecraft on Windows, real-GPU performance and FPS gains, clicking through the GUI, Minecraft 26.x launch (the game started but could not create a renderer under the CI virtual display), HUD on 1.13+.
Accounts: offline only.
