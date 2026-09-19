<script lang="ts">
  import type { NetworkRank } from "$lib/types";

  let { rank, compact = false }: { rank: NetworkRank; compact?: boolean } = $props();

  const iconPaths: Record<string, string> = {
    supporter: "/ranks/supporter.png",
    content: "/ranks/content.png",
    builder: "/ranks/builder.png",
    player: "/ranks/player.png",
    ninja: "/ranks/ninja.png",
    vip: "/ranks/vip.png",
    moderator: "/ranks/moderator.png",
    admin: "/ranks/admin.png",
    cam: "/ranks/cam.png",
  };
  const glyphIcons: Record<string, string> = {
    "⺟": "supporter",
    "⻄": "content",
    "⿔": "builder",
    "⻑": "player",
    "⻦": "ninja",
    "⪞": "vip",
    "⫸": "moderator",
    "⏅": "admin",
    "⚺": "cam",
  };

  function resolveIcon(current: NetworkRank): string | null {
    if (current.icon && iconPaths[current.icon]) return iconPaths[current.icon];
    const encodedPrefix = `${current.displayName} ${current.prefix}`;
    for (const [glyph, icon] of Object.entries(glyphIcons)) {
      if (encodedPrefix.includes(glyph)) return iconPaths[icon];
    }
    const group = current.name.toLowerCase().replace(/[^a-z]/g, "");
    if (iconPaths[group]) return iconPaths[group];
    if (group === "mod") return iconPaths.moderator;
    if (group === "con" || group === "creator") return iconPaths.content;
    if (group === "sup") return iconPaths.supporter;
    return null;
  }

  function rankColor(current: NetworkRank): string {
    return current.color && /^#[0-9a-f]{6}$/i.test(current.color) ? current.color : "#3b9df5";
  }

  let icon = $derived(resolveIcon(rank));
  let color = $derived(rankColor(rank));
</script>

<span
  class="inline-flex shrink-0 items-center justify-center overflow-hidden rounded-[4px] {compact
    ? 'min-h-4 px-0.5'
    : 'min-h-5 border px-1.5 py-0.5 shadow-[inset_0_1px_0_rgba(255,255,255,.08)]'}"
  style={compact
    ? undefined
    : `color: ${color}; border-color: ${color}55; background-color: ${color}18`}
  title={`${rank.displayName}${rank.primary ? " · Primary rank" : ""}`}
>
  {#if icon}
    <img
      src={icon}
      alt={`${rank.displayName} rank`}
      class="h-[14px] w-auto max-w-none mix-blend-screen [image-rendering:pixelated]"
    />
  {:else}
    <span class="inline-flex items-center gap-1 text-[8px] font-bold">
      <span class="size-1.5" style={`background-color: ${color}; box-shadow: 0 0 7px ${color}`}
      ></span>
      {rank.displayName}
    </span>
  {/if}
</span>
