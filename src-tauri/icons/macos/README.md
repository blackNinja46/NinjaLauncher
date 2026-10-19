# Adaptive macOS app icon

`NinjaLauncher.icon` is the original native Icon Composer document used by macOS 26 and newer. Xcode compiles it into `compiled/Assets.car` and `compiled/NinjaLauncher.icns`.

The operating system renders the layered source in Default, Dark, Tinted, and Clear appearances. The six PNG files under `appearances/` are the unchanged 1024 px exports from the original Icon Composer document. The tint color itself remains controlled by the user's macOS customization.

Regenerate the artwork and compiled assets on macOS with Xcode 26 or newer:

```sh
bun run icons:macos
```
