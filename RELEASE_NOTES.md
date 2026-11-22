Horizon Rust Clean v0.3.1 (test build / نسخة تجريبية)

Install: download **HorizonRustClean-Setup.exe** (Windows x64). Not code-signed, so SmartScreen may warn.

What was really tested in CI (details in STATUS.md):
- Actual game launch (window + still running + screenshot): Vanilla 1.8.9 / 1.12.2 / 1.16.5 / 1.20.1 / 1.21.1; Fabric 1.20.1 / 1.21.1; Quilt 1.20.1; Forge 1.12.2 / 1.16.5 / 1.20.1; NeoForge 1.21.1 — on Linux with software rendering.
- Java Runtime downloaded automatically from Mojang in every launch.
- Sodium / Lithium from Modrinth loaded in Fabric 1.20.1.
- Instances are isolated (Vanilla, Fabric, Quilt on the same version side by side).
- Windows: silent install, files present, CLI works, GUI starts.

NOT tested: launching on Windows, FPS gains, clicking through the GUI, the meta.fabricmc.net error on one user's PC (Settings -> Network check helps diagnose it), HUD on 1.13+.
Accounts: offline only.
