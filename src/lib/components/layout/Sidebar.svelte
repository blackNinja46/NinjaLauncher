<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import MinecraftAvatar from "$lib/components/account/MinecraftAvatar.svelte";
  import RankBadge from "$lib/components/account/RankBadge.svelte";
  import {
    hasNinjaPlus,
    isNameGradientUnlocked,
    nameGradient,
    rankLogoColor,
  } from "$lib/nameGradients";
  import type { Account, NavigationItem, NetworkRank, PlayerRanks } from "$lib/types";
  let {
    active,
    account,
    rank,
    networkRanks,
    halloweenTheme,
    modCount,
    onnavigate,
    onaccount,
  }: {
    active: NavigationItem;
    account: Account | null;
    rank: NetworkRank | null;
    networkRanks: PlayerRanks | null;
    halloweenTheme: boolean;
    modCount: number | null;
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

  let selectedGradientId = $state("none");
  let selectedGradient = $derived(nameGradient(selectedGradientId));
  let playerRankLogoColor = $derived(rankLogoColor(networkRanks));
  let playerHasNinjaPlus = $derived(hasNinjaPlus(networkRanks));

  function refreshGradient() {
    const owner = account?.uuid;
    if (!owner) {
      selectedGradientId = "none";
      return;
    }
    const stored = localStorage.getItem(`ninja-launcher:name-gradient:${owner}`) ?? "none";
    const gradient = nameGradient(stored);
    selectedGradientId = isNameGradientUnlocked(gradient, networkRanks) ? gradient.id : "none";
  }

  $effect(() => {
    account?.uuid;
    networkRanks;
    if (typeof window !== "undefined") refreshGradient();
  });

  onMount(() => {
    const handleGradientChange = (event: Event) => {
      const detail = (event as CustomEvent<{ uuid: string }>).detail;
      if (detail?.uuid === account?.uuid) refreshGradient();
    };
    window.addEventListener("ninja-name-gradient-changed", handleGradientChange);
    return () => window.removeEventListener("ninja-name-gradient-changed", handleGradientChange);
  });
</script>

<aside
  class:halloween-sidebar={halloweenTheme}
  class="app-sidebar relative m-3 mr-0 flex h-[calc(100%-1.5rem)] w-[236px] shrink-0 flex-col overflow-hidden rounded-[11px] border border-white/[0.12] bg-ink-950/50 px-3 pb-3 pt-4 shadow-[0_24px_70px_rgba(0,0,0,.28),inset_0_1px_0_rgba(255,255,255,.12)] backdrop-blur-sm"
>
  {#if halloweenTheme}
    <div class="halloween-sidebar-art pointer-events-none absolute inset-0" aria-hidden="true">
      <span class="halloween-moon"><Icon name="moon" size={34} /></span>
      <span class="halloween-ghost"><Icon name="ghost" size={24} /></span>
      <span class="halloween-spider"><Icon name="spider" size={18} /></span>
      <span class="halloween-cobweb"></span>
    </div>
  {/if}
  <div class="relative mb-5 flex items-center gap-2.5 border-b border-white/[0.09] px-1.5 pb-4">
    <span
      class="sidebar-brand-shine pointer-events-none relative h-14 w-16 shrink-0 select-none"
      aria-hidden="true"
      ondragstart={(event) => event.preventDefault()}
    >
      <img
        src="/brand/ninja-launcher-sidebar-logo.png"
        alt=""
        draggable="false"
        class="size-full object-contain drop-shadow-[0_5px_12px_rgba(0,0,0,.38)]"
      />
    </span>
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
        {#if item.id === "mods" && modCount !== null}<span
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
        <div class="flex min-w-0 items-center gap-1.5">
          {#if account}
            <span class="relative grid size-5 shrink-0 place-items-center" aria-hidden="true">
              <span
                class="sidebar-rank-logo absolute inset-0"
                style={`--sidebar-rank-color: ${playerRankLogoColor}`}
              ></span>
              {#if playerHasNinjaPlus}
                <span
                  class="absolute right-0 bottom-[1px] z-10 text-[10px] font-black leading-none text-white drop-shadow-[0_1px_1px_rgba(0,0,0,.95)]"
                  >+</span
                >
              {/if}
            </span>
          {/if}
          <div
            class:sidebar-name-gradient={Boolean(account) && selectedGradient.id !== "none"}
            class="min-w-0 truncate text-xs font-semibold text-[var(--color-text-primary)]"
            style={!account || selectedGradient.id === "none"
              ? undefined
              : `--sidebar-gradient-colors: ${selectedGradient.colors.join(", ")}; --sidebar-gradient-glow: ${selectedGradient.colors[1]}`}
          >
            {account?.username ?? "Not signed in"}
          </div>
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

<style>
  .sidebar-rank-logo {
    background-color: var(--sidebar-rank-color);
    filter: drop-shadow(0 1px 5px color-mix(in srgb, var(--sidebar-rank-color) 34%, transparent));
    -webkit-mask: url("/brand/ninja-launcher-app-icon.png") center / contain no-repeat;
    mask: url("/brand/ninja-launcher-app-icon.png") center / contain no-repeat;
  }

  .sidebar-brand-shine::after {
    position: absolute;
    inset: 0;
    content: "";
    pointer-events: none;
    background: linear-gradient(
      110deg,
      transparent 28%,
      rgba(255, 255, 255, 0.12) 40%,
      rgba(255, 255, 255, 0.98) 49%,
      rgba(142, 229, 255, 0.46) 55%,
      transparent 68%
    );
    background-position: 160% 50%;
    background-size: 260% 100%;
    mix-blend-mode: screen;
    -webkit-mask: url("/brand/ninja-launcher-sidebar-logo.png") center / contain no-repeat;
    mask: url("/brand/ninja-launcher-sidebar-logo.png") center / contain no-repeat;
    animation: sidebar-logo-shine 4.8s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  }

  .halloween-sidebar {
    border-color: rgb(255 138 31 / 0.22);
    background:
      linear-gradient(180deg, rgb(20 10 27 / 0.76), rgb(8 5 11 / 0.88)),
      var(--color-section-background);
    box-shadow:
      0 24px 70px rgb(0 0 0 / 0.42),
      inset 0 1px 0 rgb(255 153 57 / 0.14),
      inset 0 0 70px rgb(93 35 144 / 0.08);
  }

  .halloween-sidebar-art {
    z-index: 0;
    overflow: hidden;
  }

  .halloween-moon {
    position: absolute;
    top: 104px;
    right: -7px;
    color: rgb(255 151 54 / 0.18);
    filter: drop-shadow(0 0 15px rgb(255 117 24 / 0.32));
  }

  .halloween-ghost {
    position: absolute;
    right: 17px;
    bottom: 112px;
    color: rgb(192 137 255 / 0.18);
    filter: drop-shadow(0 0 12px rgb(133 64 215 / 0.28));
    animation: halloween-ghost-float 4.8s ease-in-out infinite;
  }

  .halloween-spider {
    position: absolute;
    top: 193px;
    left: 11px;
    color: rgb(255 140 35 / 0.2);
    transform-origin: 50% -32px;
    animation: halloween-spider-sway 4s ease-in-out infinite;
  }

  .halloween-spider::before {
    position: absolute;
    bottom: 12px;
    left: 50%;
    width: 1px;
    height: 38px;
    content: "";
    background: linear-gradient(transparent, rgb(255 147 47 / 0.24));
  }

  .halloween-cobweb {
    position: absolute;
    top: -26px;
    right: -28px;
    width: 105px;
    height: 105px;
    opacity: 0.11;
    background:
      repeating-radial-gradient(circle at 100% 0%, transparent 0 12px, #ff9a38 13px 14px),
      conic-gradient(
        from 180deg at 100% 0%,
        transparent 0 9deg,
        #ff9a38 10deg 11deg,
        transparent 12deg 34deg,
        #ff9a38 35deg 36deg,
        transparent 37deg 59deg,
        #ff9a38 60deg 61deg,
        transparent 62deg 89deg,
        #ff9a38 90deg 91deg,
        transparent 92deg
      );
  }

  .sidebar-name-gradient {
    color: transparent;
    background-image: linear-gradient(90deg, var(--sidebar-gradient-colors));
    background-size: 160% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    filter: saturate(1.25) brightness(1.1)
      drop-shadow(0 1px 4px color-mix(in srgb, var(--sidebar-gradient-glow) 30%, transparent));
    animation: sidebar-name-gradient-flow 1.6s ease-in-out infinite alternate;
  }

  @keyframes sidebar-name-gradient-flow {
    from {
      background-position: 0% 50%;
    }
    to {
      background-position: 100% 50%;
    }
  }

  @keyframes sidebar-logo-shine {
    0%,
    58% {
      background-position: 160% 50%;
    }
    82%,
    100% {
      background-position: -80% 50%;
    }
  }

  @keyframes halloween-ghost-float {
    0%,
    100% {
      transform: translateY(3px) rotate(-4deg);
      opacity: 0.62;
    }
    50% {
      transform: translateY(-7px) rotate(3deg);
      opacity: 1;
    }
  }

  @keyframes halloween-spider-sway {
    0%,
    100% {
      transform: rotate(-5deg);
    }
    50% {
      transform: rotate(6deg);
    }
  }

  :global(.animations-disabled) .sidebar-name-gradient,
  :global(.animations-disabled) .sidebar-brand-shine::after,
  :global(.animations-disabled) .halloween-ghost,
  :global(.animations-disabled) .halloween-spider {
    animation: none;
    background-position: 50% 50%;
  }

  @media (prefers-reduced-motion: reduce) {
    .sidebar-name-gradient,
    .sidebar-brand-shine::after,
    .halloween-ghost,
    .halloween-spider {
      animation: none;
    }
  }
</style>
