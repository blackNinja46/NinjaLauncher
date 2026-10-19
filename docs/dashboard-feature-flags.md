# Dashboard feature flags

The launcher reads feature flags from `configuration.enabledFeatures` in the client bootstrap response.

## Halloween theme

- Flag key: `halloween_theme`
- Enabled: the Halloween theme and ambient-sound toggles are shown under **Settings → Appearance & accessibility**. Both default to on for users who have not chosen a preference yet.
- Disabled or missing: the toggle is hidden and the seasonal theme is not applied, even if it was previously enabled locally.

This lets the Dashboard control the seasonal availability without overwriting the user's saved local preference. If the flag is enabled again later, the user's previous choice becomes active again.

The first time the 2026 event is available, the launcher shows a short introduction. That state is stored locally under `ninja-launcher:halloween-event:2026:intro-seen`; use a new event-specific key for a future seasonal introduction.

Ambient sounds begin only after 45 seconds without launcher input. They stop immediately when the user interacts, when Minecraft starts, or when the launcher is no longer the focused visible window.
