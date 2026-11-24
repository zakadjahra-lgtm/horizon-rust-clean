MClauncher v1.1.0 (test build / نسخة تجريبية)

Install: download **MClauncher-Setup.exe** (Windows x64). Not code-signed, so SmartScreen may warn. Data folder: `%APPDATA%\MClauncher`.

New since v1.0.0:
- **Play page: Clients section** — Horizon Client (our HUD client), Badlion Client, Lunar Client and custom clients. The Badlion/Lunar/custom cards store the path of the client's own program and start it; their versions, mods and HUD stay in their own launchers. No third-party logos.
- **Rust Engine page** (sidebar), separate from Java instances and from clients: own folder, graphics settings (seed, view radius, VSync), saved world, Launch button. It is the Rust prototype game only: no Horizon mods, no version list beyond "Prototype".
- **Browse**: results now show project icons and authors; Modrinth and CurseForge (CurseForge needs your own API key in Settings).
- The Java client environment is named "Horizon Client" again.

Verified by CI (STATUS.md): launches of Vanilla/Fabric/Quilt/Forge/NeoForge on Linux, **Minecraft 26.3 launch on Linux** (Vulkan software renderer; one run), instance isolation, Windows installer + GUI start, unit tests on Linux and Windows.
Reported by the maintainer (not CI): Minecraft 26.3 starts from the launcher on Windows and reaches the Mojang loading screen.
NOT tested: the new Clients and Rust Engine pages in the running app, starting an external client, CurseForge against the real API, real-GPU performance.
Accounts: offline only.
