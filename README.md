# Ninja Launcher

A modern cross-platform Minecraft Java Edition launcher built with Tauri 2, Rust,
Svelte 5, TypeScript, Tailwind CSS, and Bun.

## Current capabilities

- Live Modrinth search filtered by Minecraft version and Fabric
- Persistent isolated instances and managed mod synchronization
- Required Modrinth dependency resolution
- Microsoft device-code sign-in and Minecraft ownership/profile verification
- Mojang client, library and asset installation
- Fabric profile installation, Java detection and Minecraft process launch

Microsoft sign-in needs a public desktop application registration. Create one in
Microsoft Entra, enable public client flows, and enter its Application (client) ID
under **Settings → Microsoft OAuth client ID**. No client secret is used or stored.

## Development

```sh
bun install
bun run check
bun run tauri dev
```

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
