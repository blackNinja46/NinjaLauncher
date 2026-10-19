import type { SupportedVersion } from "$lib/types";

const minecraftVersionCollator = new Intl.Collator("en", {
  numeric: true,
  sensitivity: "base",
});

export function compareMinecraftVersionsDescending(left: string, right: string): number {
  return minecraftVersionCollator.compare(right, left);
}

export function sortSupportedVersions(versions: SupportedVersion[]): SupportedVersion[] {
  return [...versions].sort((left, right) => {
    const byVersion = compareMinecraftVersionsDescending(
      left.minecraftVersion,
      right.minecraftVersion,
    );
    if (byVersion !== 0) return byVersion;
    return Number(left.versionType === "snapshot") - Number(right.versionType === "snapshot");
  });
}
