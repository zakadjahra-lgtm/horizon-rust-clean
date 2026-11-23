# Horizon Rust Clean — plan and honest scope

**ملخص بالعربية:** هذا المشروع تطوير (fork) فوق Horizon Offline Launcher الأصلي (MIT)، وليس لانشر جديد. Horizon لعبة Minecraft 26.3 مُعاد كتابتها بـ Rust،
وليست مشغّل Java. لا تشغّل Minecraft Java الأصلية ولا مودات Forge/Fabric. معظم أدوات الـ HUD المطلوبة موجودة أصلاً في المشروع؛ نضيف ما ينقص فعلاً ونوثّق
ما يتطلب تطويراً جذرياً.

## What this repository really is (from reading the code, not only the README)

| Part | Where | Notes |
|---|---|---|
| The game | `crates/horizon-client` (+ `-render`, `-input`, `-core`, `horizon-world`, `-items`, `-protocol`, ...) | Minecraft Java 26.3 rewritten in Rust (wgpu). Plays vanilla 26.3 worlds. 40 crates, ~2700 Rust files. Vanilla is the spec; parity harnesses compare against the real game. |
| The launcher | `launcher/` (own Cargo workspace, GPUI UI) | Offline account, own skin/cape, "Choose jar...", updates from the repository's signed releases. |
| Mods | `crates/horizon-mods`, `docs/mods.md`, `docs/native-mods.md` | `.hmod` content mods (data/assets, no code) and `.hnmod` native mods (Rust mixins that rebuild the game). Fabric/NeoForge jars do **not** run; only their data/assets can be ported (`horizon-port-pack`). |
| Design tokens | `crates/horizon-theme`, `docs/design-system.md` | One source for launcher, in-game UI and website. |
| OptiFine-format packs | `crates/horizon-optifine` | CTM, CIT, emissive/random entities, custom sky/colours. |

The game needs the user's own Minecraft 26.3 files (it never ships Mojang assets). Releases are signed with the repository's key (`RELEASE_SIGNING_KEY`).

## Requested features vs what already exists

| Requested | Status in the original code |
|---|---|
| FPS counter | **Exists** (Options > PvP & HUD > FPS, with corner) |
| CPS counter | **Exists** (`game/pvp.rs`, CPS widget) |
| Keystrokes | **Exists** (`game/pvp.rs`, Keystrokes widget) |
| Toggle sprint / sneak | **Exists** (`settings.toggle_sprint`, `toggle_crouch`) |
| Armor status | **Exists** ("Armour", durability or %, corner) |
| Effects, ping, coordinates, combo, clock, reach, speed, held-item count, custom crosshair, HUD editor | **Exist** |
| Zoom | **Only as a native mod hook** (`native_zoom`); built-in zoom is missing -> candidate for Phase 2 |
| "Clean" look inspired by Lunar/Badlion | Partly (HUD widgets + PvP preset). A one-click Clean layout/theme is missing -> Phase 2 |

We do not re-implement what exists. We add what is missing and test it.

## What can be added (real, bounded work)

- Built-in zoom (hold key, scroll to adjust, smoothing) reusing the existing zoom path in `game/extras.rs`.
- A "Clean" preset: HUD widget layout and defaults in the style of Lunar/Badlion, using the existing widgets.
- A Clean launcher theme from `horizon-theme` tokens.
- Performance: first a repeatable measurement (`--benchmark standard`, `docs/benchmark.md`), then targeted changes backed by numbers. No FPS gain is claimed without a measurement.
- More content-mod tooling on top of `.hmod`/modpack porting.

## What needs radical development (not promised)

- Running original Minecraft Java, or Java mods (Forge/Fabric/Quilt/NeoForge): Horizon contains no JVM and none of Mojang's classes. A different product (our earlier Java launcher, kept separately) does that.
- Other Minecraft versions (1.8.9, 1.12.2, 1.20.1...): Horizon's data, worldgen and network protocol are ported from exactly 26.3, class by class. Each version would be a new port.
- Loading Fabric/Forge code mods: impossible without Java; data/assets of such mods can be converted by `horizon-port-pack` only.
- Shipping a playable build from this fork: needs our own release signing key and launcher public key (the original keys are not ours) and the prepared Horizon resources. Planned, not done.

## Licence

MIT. `LICENSE` keeps the upstream copyright lines and adds ours; `THIRD_PARTY_NOTICES.md` is unchanged. We never commit Mojang files (`tools/check_public_tree.py`).

## Phases

1. **Fork and baseline (this commit):** audit, notices, plan, a CI job that builds the launcher and the game crate so every later change is compiled and tested.
2. Built-in Zoom + Clean preset (game), with unit tests where logic is pure.
3. Clean launcher theme.
4. Measured performance work.
5. Our own signing key and a playable build channel.
