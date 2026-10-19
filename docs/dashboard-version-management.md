# Dashboard-controlled Minecraft versions

The launcher consumes the `minecraftVersions` array returned by:

`GET /api/v1/client/bootstrap`

Each entry has this shape:

```json
{
  "version": "1.21.11",
  "recommended": true,
  "releaseType": "RELEASE",
  "loaders": [
    { "type": "FABRIC", "version": "0.18.4" }
  ]
}
```

## Launcher behavior

- Every Minecraft version returned by the dashboard is automatically available to and controls Vanilla profiles. Vanilla does not require a separate loader entry.
- Fabric profiles only receive versions that contain a `FABRIC` loader entry.
- Versions missing from the managed loader list are hidden from profile creation, profile editing, and the home version picker.
- Existing profiles using a disabled version cannot launch until another enabled version is selected.
- The Fabric loader version is taken from the matching `FABRIC` entry.
- Entries marked `recommended` are placed first and therefore become the default for newly created profiles.
- `SNAPSHOT` entries still respect the launcher's local **Show snapshots** preference.
- If the dashboard returns no Minecraft versions, the launcher retains its locally discovered catalogs as a compatibility fallback.

The dashboard should expose create, edit, delete, recommended, release type, and optional loader assignment controls for these records. Deleting every Minecraft version restores the local fallback; use maintenance mode when every version must be blocked.
