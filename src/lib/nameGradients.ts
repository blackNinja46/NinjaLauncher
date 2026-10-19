import type { NetworkRank, PlayerRanks } from "$lib/types";

export interface NameGradient {
  id: string;
  label: string;
  rankName: string | null;
  tier: number;
  colors: [string, string, string];
}

export const nameGradients: NameGradient[] = [
  {
    id: "none",
    label: "Classic",
    rankName: null,
    tier: -1,
    colors: ["#ffffff", "#ffffff", "#ffffff"],
  },
  {
    id: "ninja",
    label: "Ninja+",
    rankName: "ninja-plus",
    tier: 1,
    colors: ["#6bf1ff", "#1976e9", "#24d9ff"],
  },
  {
    id: "vip",
    label: "VIP",
    rankName: "vip",
    tier: 2,
    colors: ["#ff7ce8", "#a855f7", "#ff9ee7"],
  },
  {
    id: "builder",
    label: "Builder",
    rankName: "builder",
    tier: 3,
    colors: ["#d8ff78", "#2fd06f", "#9cff57"],
  },
  {
    id: "team",
    label: "Team",
    rankName: "team",
    tier: 4,
    colors: ["#ff6b7d", "#ff1744", "#ff3d68"],
  },
];

function normalizedRankName(value: string): string {
  return value
    .toLowerCase()
    .replaceAll("+", "plus")
    .replace(/[^a-z]/g, "");
}

export function rankTier(value: string): number {
  const name = normalizedRankName(value);
  if (name.includes("admin")) return 7;
  if (name.includes("moderator") || name === "mod") return 6;
  if (name.includes("content") || name === "con" || name.includes("creator")) return 5;
  if (name.includes("supporter") || name === "support" || name === "sup") return 4;
  if (name.includes("builder") || name === "build") return 3;
  if (name.includes("vip")) return 2;
  if (name === "ninjaplus") return 1;
  if (name.includes("player") || name === "default") return 0;
  return -1;
}

export function highestUnlockedGradientTier(ranks: PlayerRanks | null): number {
  if (!ranks) return 0;
  return Math.max(
    0,
    rankTier(ranks.primaryGroup),
    ...ranks.ranks.flatMap((rank: NetworkRank) => [
      rankTier(rank.name),
      rankTier(rank.displayName),
    ]),
  );
}

function assignedRankTiers(ranks: PlayerRanks | null): Set<number> {
  if (!ranks) return new Set([0]);
  return new Set([
    rankTier(ranks.primaryGroup),
    ...ranks.ranks.flatMap((rank: NetworkRank) => [
      rankTier(rank.name),
      rankTier(rank.displayName),
    ]),
  ]);
}

export function isNameGradientUnlocked(gradient: NameGradient, ranks: PlayerRanks | null): boolean {
  if (gradient.id === "none") return true;
  const tiers = assignedRankTiers(ranks);
  if (tiers.has(7)) return true;
  if (gradient.id === "team") return tiers.has(4) || tiers.has(5) || tiers.has(6);
  return tiers.has(gradient.tier);
}

const rankLogoColors = [
  "#d5dbe5",
  "#00c8ff",
  "#ff3fe1",
  "#49ee53",
  "#ff314f",
  "#ff314f",
  "#ff314f",
  "#ff314f",
] as const;

export function rankLogoColor(ranks: PlayerRanks | null): string {
  const tier = highestUnlockedGradientTier(ranks);
  return rankLogoColors[Math.min(Math.max(tier, 0), rankLogoColors.length - 1)];
}

export function hasNinjaPlus(ranks: PlayerRanks | null): boolean {
  if (!ranks) return false;
  const labels = [
    ranks.primaryGroup,
    ...ranks.ranks.flatMap((rank) => [rank.name, rank.displayName]),
  ];
  return labels.some((label) => {
    const normalized = normalizedRankName(label);
    const tier = rankTier(label);
    return normalized === "ninjaplus" || tier === 2 || tier === 7;
  });
}

export function nameGradient(id: string): NameGradient {
  const migratedId =
    id === "player"
      ? "none"
      : ["supporter", "content", "moderator", "admin"].includes(id)
        ? "team"
        : ["ninja-plus", "ninjaplus"].includes(id)
          ? "ninja"
          : id;
  return nameGradients.find((gradient) => gradient.id === migratedId) ?? nameGradients[0];
}
