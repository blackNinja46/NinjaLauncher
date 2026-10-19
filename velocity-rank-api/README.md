# Ninja Launcher Rank API

Read-only Velocity plugin for exposing effective LuckPerms groups to Ninja Launcher. The plugin
uses the LuckPerms API, so the existing MySQL credentials remain only in LuckPerms' configuration.

## Build and install

1. Build the plugin with `./velocity-rank-api/gradlew -p velocity-rank-api build`.
2. Copy `velocity-rank-api/build/libs/ninja-launcher-rank-api-1.0.0.jar` into the Velocity
   `plugins` directory.
3. Restart Velocity. The generated config is stored in
   `plugins/ninja-launcher-rank-api/config.properties`.
4. Keep `bind-address=127.0.0.1`. Expose the endpoint through the existing HTTPS reverse proxy.

Example nginx location:

```nginx
location /api/launcher/ranks/ {
    proxy_pass http://127.0.0.1:8765/api/launcher/ranks/;
    proxy_set_header X-Real-IP $remote_addr;
    limit_req zone=launcher_rank_api burst=10 nodelay;
}
```

The launcher uses `https://blackninja.live/api/launcher/ranks/{minecraft-uuid}` by default. Override
the base URL at build time with `NINJA_RANK_API_URL` if the public route differs.

## LuckPerms metadata

For the best display, set these group metadata values:

- `rank-name` (preferred) or `display-name`: readable label shown in the launcher. Labels that
  contain only icon glyphs fall back to the group name.
- `rank-color`: named Minecraft color or `#RRGGBB`
- group weight: controls rank order

The resource-pack prefix glyphs for supporter, content, builder, player, ninja, VIP, moderator,
admin and CAM are mapped to the matching bitmap automatically. Group names are used as a fallback.

The `default` group is hidden by default. Change `hidden-groups` to a comma-separated list if needed.
