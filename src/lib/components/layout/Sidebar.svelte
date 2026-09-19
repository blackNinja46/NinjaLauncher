<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";
  import MinecraftAvatar from "$lib/components/account/MinecraftAvatar.svelte";
  import RankBadge from "$lib/components/account/RankBadge.svelte";
  import type { Account, NavigationItem, NetworkRank } from "$lib/types";
  let {
    active,
    account,
    rank,
    modCount,
    onnavigate,
    onaccount,
  }: {
    active: NavigationItem;
    account: Account | null;
    rank: NetworkRank | null;
    modCount: number;
    onnavigate: (page: NavigationItem) => void;
    onaccount: () => void;
  } = $props();
  const items: { id: NavigationItem; label: string }[] = [
    { id: "home", label: "Home" },
    { id: "skins", label: "Skins" },
    { id: "mods", label: "Mods" },
    { id: "modpacks", label: "Modpacks" },
    { id: "instances", label: "Profiles" },
    { id: "logs", label: "Logs" },
    { id: "settings", label: "Settings" },
  ];
</script>

<aside
  class="relative m-3 mr-0 flex h-[calc(100%-1.5rem)] w-[236px] shrink-0 flex-col overflow-hidden rounded-[11px] border border-white/[0.12] bg-ink-950/50 px-3 pb-3 pt-4 shadow-[0_24px_70px_rgba(0,0,0,.28),inset_0_1px_0_rgba(255,255,255,.12)] backdrop-blur-sm"
>
  <div class="relative mb-5 flex items-center gap-3 border-b border-white/[0.09] px-2 pb-4">
    <img
      src="/brand/ninja-launcher-mark.png"
      alt="Ninja Launcher NL logo"
      class="h-10 w-11 object-contain"
    />
    <div>
      <div class="block-title text-[15px] tracking-[-.02em]">Ninja Launcher</div>
      <div
        class="mt-1 text-[9px] font-bold uppercase tracking-[.16em] text-[var(--color-brand-sky)]"
      >
        Java Edition
      </div>
    </div>
  </div>
  <nav class="relative space-y-1" aria-label="Main navigation">
    {#each items as item}
      <button
        onclick={() => onnavigate(item.id)}
        class="group flex w-full items-center gap-3 rounded-[6px] border px-3 py-2.5 tracking-[-.01em] {active ===
        item.id
          ? 'border-white/[.15] bg-white/[.12] font-bold text-white shadow-[inset_0_1px_0_rgba(255,255,255,.1),0_8px_24px_rgba(0,0,0,.12)]'
          : 'border-transparent text-[var(--color-text-muted)] hover:border-white/[.08] hover:bg-white/[0.065] hover:text-white bold'}"
      >
        <span class="transition-transform duration-300 group-hover:scale-110"
          ><Icon name={item.id} size={18} /></span
        ><span>{item.label}</span>
        {#if active === item.id}{/if}
        {#if item.id === "mods"}<span
            class="{active === item.id
              ? ''
              : 'ml-auto'} rounded-full text-left bg-white/[0.08] px-1.5 py-0.5 font-bold text-[10px] text-[var(--color-text-secondary)]"
            >{modCount}</span
          >{/if}
      </button>
    {/each}
  </nav>
  <div class="relative mt-auto border-t border-white/[0.09] pt-3">
    <button onclick={onaccount} class="mc-secondary flex w-full items-center gap-3 p-2 text-left">
      <div class="size-9 shrink-0 rounded-[4px] text-[11px] drop-shadow-lg">
        {#key account?.uuid}
          <MinecraftAvatar {account} />
        {/key}
      </div>
      <div class="min-w-0 flex-1">
        <div class="truncate text-xs font-semibold text-[var(--color-text-primary)]">
          {account?.username ?? "Not signed in"}
        </div>
        <div
          class="flex min-h-4 items-center truncate text-[10px] font-medium text-[var(--color-text-muted)]"
        >
          {#if account && rank}
            <RankBadge {rank} compact />
          {:else}
            {account ? "Minecraft account" : "Microsoft account"}
          {/if}
        </div>
      </div>
      <Icon name="chevron" size={14} />
    </button>
  </div>
</aside>
