# Third-party notices

Horizon Rust Clean does not contain code from Lunar Client, Badlion Client, Apollo, Sodium, Lithium or Mojang's launcher code.

## Shipped inside `horizon-client.jar` (Java agent)

| Component | License | Notes |
|---|---|---|
| [ASM](https://asm.ow2.io/) 9.7 (`org.ow2.asm:asm`) | BSD-3-Clause | Shaded and relocated to `dev.horizon.client.shaded.asm`. Copyright © 2000–2011 INRIA, France Telecom. Redistribution requires keeping this notice and the BSD-3-Clause terms. |

## Used at runtime, not redistributed by us

- Minecraft files (client, libraries, assets) and Java runtimes are downloaded by the user's launcher directly from Mojang's official servers.
- Optional mods (e.g. Sodium, Lithium) are downloaded from Modrinth on the user's request and keep their own licenses.

## Build-time dependencies

Rust crates (reqwest, tokio, serde, tauri, …) are under MIT and/or Apache-2.0. A full generated list (e.g. with `cargo about`) should be produced before a public release.
