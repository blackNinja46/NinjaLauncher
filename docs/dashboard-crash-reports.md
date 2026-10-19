# Dashboard crash-report endpoint

The launcher submits a report only after the player clicks **Upload to dashboard** in the crash dialog.

## Endpoint

`POST /api/v1/client/crash-reports`

The endpoint must be available to launcher clients without a dashboard administrator session. It should apply request-size limits and rate limiting. Any `2xx` response is treated as success; `204 No Content` is sufficient.

## JSON request

```json
{
  "clientReportId": "3f3cd9d2-6b9d-40cd-ad51-84f071bdbcb8",
  "launcherVersion": "2.0.0-beta.1",
  "operatingSystem": "MACOS",
  "profileId": "default-fabric",
  "profileName": "My Fabric World",
  "minecraftVersion": "26.3",
  "loader": "fabric",
  "accountUuid": "optional Minecraft UUID",
  "accountName": "optional Minecraft name",
  "exitCode": null,
  "title": "Incompatible mod",
  "message": "Human-readable summary",
  "suggestion": "Recommended next step",
  "technicalDetail": "Optional detected error",
  "logExcerpt": ["[stderr] final relevant console line"]
}
```

The backend should use `clientReportId` as an idempotency key so retrying a request cannot create duplicate reports. Timestamps should be assigned by the backend rather than trusted from the client.

## Limits enforced by the launcher

- Up to 200 log lines and 128 KB of log text
- Up to 32 KB of technical details
- Up to 4 KB each for the summary and recommendation
