<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";
  import MinecraftAvatar from "$lib/components/account/MinecraftAvatar.svelte";
  import type { NavigationItem } from "$lib/types";
  import type { Account } from "$lib/types";
  let {
    active,
    account,
    modCount,
    onnavigate,
    onaccount,
  }: {
    active: NavigationItem;
    account: Account | null;
    modCount: number;
    onnavigate: (page: NavigationItem) => void;
    onaccount: () => void;
  } = $props();
  const items: { id: NavigationItem; label: string }[] = [
    { id: "home", label: "Home" },
    { id: "mods", label: "Mods" },
    { id: "modpacks", label: "Modpacks" },
    { id: "instances", label: "Profiles" },
    { id: "logs", label: "Logs" },
    { id: "settings", label: "Settings" },
  ];
</script>

<aside
  class="relative flex h-full w-[232px] shrink-0 flex-col overflow-hidden border-r border-cyan-300/[0.13] bg-[#020817] px-3 pb-4 pt-5 shadow-[8px_0_32px_rgba(0,5,20,.25)]"
>
  <div class="pixel-grid pointer-events-none absolute inset-0 opacity-70"></div>
  <div class="relative mb-7 flex items-center gap-3 border-b border-cyan-300/[0.1] px-3 pb-5">
    <div
      class="grid size-11 shrink-0 place-items-center overflow-hidden bg-[#031237] shadow-[0_0_18px_rgba(9,159,240,.2)]"
    >
      <img
        src="/brand/ninja-launcher-mark.png"
        alt="Ninja Launcher NL logo"
        class="h-10 w-11 object-contain"
      />
    </div>
    <div>
      <div class="block-title text-[15px] uppercase tracking-[.035em]">Ninja Launcher</div>
      <div class="mt-0.5 text-[9px] font-bold uppercase tracking-[.18em] text-cyan-300/60">
        Java Edition
      </div>
    </div>
  </div>
  <nav class="relative space-y-1.5" aria-label="Main navigation">
    {#each items as item}
      <button
        onclick={() => onnavigate(item.id)}
        class="flex w-full items-center gap-3 border px-3 py-2.5 text-left text-[12px] font-bold uppercase tracking-[.04em] transition {active ===
        item.id
          ? 'border-cyan-300/25 bg-gradient-to-r from-accent-500/25 to-accent-500/[.06] text-cyan-50 shadow-[inset_3px_0_0_#19d8f1]'
          : 'border-transparent text-[#7190b0] hover:border-cyan-300/10 hover:bg-blue-400/[0.07] hover:text-white'}"
      >
        <Icon name={item.id} size={18} /><span>{item.label}</span>
        {#if item.id === "mods"}<span
            class="ml-auto bg-cyan-300/[0.08] px-1.5 py-0.5 text-[10px] text-cyan-200/65"
            >{modCount}</span
          >{/if}
      </button>
    {/each}
  </nav>
  <div class="relative mt-auto border-t border-cyan-300/[0.1] pt-4">
    <button onclick={onaccount} class="mc-secondary flex w-full items-center gap-3 p-2 text-left">
      <div class="size-8 shrink-0 text-[11px]">
        {#key account?.uuid}
          <MinecraftAvatar {account} />
        {/key}
      </div>
      <div class="min-w-0 flex-1">
        <div class="truncate text-xs font-medium text-[#d7d9de]">
          {account?.username ?? "Not signed in"}
        </div>
        <div class="text-[10px] text-[#737884]">
          {account ? "Minecraft account" : "Microsoft account"}
        </div>
      </div>
      <Icon name="chevron" size={14} />
    </button>
  </div>
</aside>
