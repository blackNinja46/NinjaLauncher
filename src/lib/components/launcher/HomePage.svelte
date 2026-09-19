<script lang="ts">
  import { onMount } from "svelte";
  import { getServerIcon } from "$lib/api/tauri";
  import RankBadge from "$lib/components/account/RankBadge.svelte";
  import PlayerSkin from "$lib/components/account/PlayerSkin.svelte";
  import CustomSelect from "$lib/components/ui/CustomSelect.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import type { Account, LauncherInstance, PlayerRanks, SupportedVersion } from "$lib/types";

  let {
    instance,
    instances,
    versions,
    signedIn,
    account,
    networkRanks,
    launching,
    running,
    stopping,
    progress,
    status,
    maintenanceActive,
    maintenanceMessage,
    onplay,
    onconnect,
    onstop,
    onopenmods,
    onversionchange,
    onprofilechange,
  }: {
    instance: LauncherInstance;
    instances: LauncherInstance[];
    versions: SupportedVersion[];
    signedIn: boolean;
    account: Account | null;
    networkRanks: PlayerRanks | null;
    launching: boolean;
    running: boolean;
    stopping: boolean;
    progress: number | null;
    status: string;
    maintenanceActive: boolean;
    maintenanceMessage: string;
    onplay: () => Promise<void>;
    onconnect: (serverAddress: string) => Promise<void>;
    onstop: () => Promise<void>;
    onopenmods: () => void;
    onversionchange: (version: string) => Promise<void>;
    onprofilechange: (id: string) => void;
  } = $props();

  let profileMenu: HTMLDivElement;
  let profileMenuOpen = $state(false);
  const featuredServers = [
    { name: "blackNinja.Live", address: "blackninja.live" },
    { name: "NinjaSMP.net", address: "ninjasmp.net" },
  ];
  let serverIcons = $state<Record<string, string>>({});
  let primaryRank = $derived(
    networkRanks?.ranks.find((rank) => rank.primary) ?? networkRanks?.ranks[0] ?? null,
  );

  onMount(() => {
    for (const server of featuredServers) {
      getServerIcon(server.address)
        .then((icon) => {
          if (icon) serverIcons = { ...serverIcons, [server.address]: icon };
        })
        .catch(() => {});
    }
  });

  function selectProfile(id: string) {
    onprofilechange(id);
    profileMenuOpen = false;
  }

  $effect(() => {
    if (!profileMenuOpen) return;
    const closeMenu = (event: PointerEvent) => {
      if (!profileMenu?.contains(event.target as Node)) profileMenuOpen = false;
    };
    const closeWithKeyboard = (event: KeyboardEvent) => {
      if (event.key === "Escape") profileMenuOpen = false;
    };
    document.addEventListener("pointerdown", closeMenu);
    document.addEventListener("keydown", closeWithKeyboard);
    return () => {
      document.removeEventListener("pointerdown", closeMenu);
      document.removeEventListener("keydown", closeWithKeyboard);
    };
  });
</script>

<section class="group">
  <div class="relative h-[clamp(480px,calc(100vh-136px),574px)] px-5 sm:px-7">
    <div class="relative flex h-full flex-col items-center">
      {#if maintenanceActive}
        <div
          class="absolute left-1/2 top-2 z-50 w-[min(92%,560px)] -translate-x-1/2 border border-amber-300/30 bg-amber-400/10 px-4 py-3 text-center shadow-[0_12px_36px_rgba(0,0,0,.35)] backdrop-blur-xl"
          role="alert"
        >
          <p class="text-[9px] font-bold uppercase tracking-[.16em] text-amber-200">Maintenance</p>
          <p class="mt-1 text-xs text-amber-50/80">
            {maintenanceMessage || "The launcher is temporarily unavailable."}
          </p>
        </div>
      {/if}
      <div
        class="pointer-events-none absolute left-1/2 top-10 z-30 flex -translate-x-1/2 flex-col items-center gap-1"
      >
        {#if primaryRank}<RankBadge rank={primaryRank} compact />{/if}
        <h1
          class="whitespace-nowrap text-[26px] font-extrabold tracking-[-.035em] text-white drop-shadow-[0_3px_12px_rgba(0,0,0,.65)]"
        >
          {account?.username ?? "Guest"}
        </h1>
      </div>

      <div class="min-h-0 w-[clamp(420px,52%,560px)] flex-1 translate-y-3 pt-20">
        <PlayerSkin {account} {launching} rank={primaryRank} showNameTag={false} />
      </div>

      <aside
        class="absolute right-0 top-1/2 z-30 flex -translate-y-1/2 flex-col gap-2.5"
        aria-label="Featured servers"
      >
        {#each featuredServers as server}
          <button
            type="button"
            onclick={() => onconnect(server.address)}
            disabled={launching || running || !signedIn || maintenanceActive}
            aria-label={`Connect to ${server.name}`}
            title={signedIn ? `Connect to ${server.name}` : "Sign in to use multiplayer"}
            class="server-logo grid size-12 place-items-center overflow-hidden rounded-[6px] border border-white/[.14] bg-[var(--color-surface-strong)] shadow-[0_10px_28px_rgba(0,4,18,.32)] backdrop-blur-xl disabled:cursor-not-allowed disabled:opacity-40"
          >
            {#if serverIcons[server.address]}
              <img
                src={serverIcons[server.address]}
                alt=""
                class="size-full object-cover [image-rendering:pixelated]"
              />
            {:else}
              <Icon name="server" size={18} />
            {/if}
          </button>
        {/each}
      </aside>

      <div bind:this={profileMenu} class="relative z-40 w-72 shrink-0 -translate-y-8 pb-2">
        <div
          class="play-split {launching
            ? 'is-launching'
            : running
              ? 'is-running'
              : 'is-ready'} flex overflow-hidden rounded-[7px] border border-cyan-100/65 text-lg font-bold text-white shadow-[0_10px_28px_var(--color-brand-glow)]"
        >
          {#if running}
            <button
              onclick={onstop}
              disabled={stopping}
              class="flex min-w-0 flex-1 items-center justify-center gap-2 bg-red-500/35 px-6 py-3.5 text-sm font-bold"
            >
              <span class="size-2.5 bg-current"></span>{stopping ? "Stopping…" : "Stop game"}
            </button>
          {:else}
            <button
              onclick={onplay}
              disabled={launching || maintenanceActive}
              class="launch-main flex min-w-0 flex-1 items-center justify-center gap-2 px-6 py-3"
            >
              <span class="launch-glyph {launching ? 'is-loading' : ''}">
                <Icon name="play" size={15} />
              </span>{maintenanceActive
                ? "Maintenance"
                : launching
                  ? "Preparing…"
                  : signedIn
                    ? "Play"
                    : "Play demo"}
            </button>
          {/if}

          <button
            type="button"
            onclick={() => (profileMenuOpen = !profileMenuOpen)}
            disabled={launching || running}
            class="grid w-14 shrink-0 place-items-center border-l border-white/35 bg-black/10 hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-45"
            aria-label="Choose profile"
            aria-haspopup="listbox"
            aria-expanded={profileMenuOpen}
            title="Choose profile"
          >
            <span
              class={"transition-transform duration-200 " + (profileMenuOpen ? "rotate-180" : "")}
            >
              <Icon name="chevron-down" size={12} />
            </span>
          </button>
        </div>

        {#if profileMenuOpen}
          <div
            class="profile-menu absolute bottom-[calc(100%+.5rem)] right-0 z-50 max-h-52 w-72 overflow-y-auto border border-accent-400/25 bg-[var(--color-surface-strong)] p-1.5 shadow-[0_22px_60px_rgba(0,4,18,.75)] backdrop-blur-3xl"
            role="listbox"
            aria-label="Launcher profiles"
          >
            {#each instances as profile (profile.id)}
              <button
                type="button"
                role="option"
                aria-selected={profile.id === instance.id}
                onclick={() => selectProfile(profile.id)}
                class="flex w-full items-center gap-3 border px-3 py-2 text-left transition-colors {profile.id ===
                instance.id
                  ? 'border-accent-400/25 bg-accent-500/15 text-white'
                  : 'border-transparent text-[var(--color-text-secondary)] hover:bg-white/[.06] hover:text-white'}"
              >
                <span
                  class="grid size-7 shrink-0 place-items-center bg-white/[.05] text-accent-400"
                >
                  <Icon name={profile.id === instance.id ? "check" : "instances"} size={11} />
                </span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-[10px] font-bold">{profile.name}</span>
                  <span class="block text-[8px] text-white/40"
                    >Minecraft {profile.minecraftVersion}</span
                  >
                </span>
              </button>
            {/each}
          </div>
        {/if}

        <div class="mt-2.5" aria-live="polite">
          {#if progress !== null}
            <div
              class="mb-1 flex items-center justify-between text-[14px] font-semibold text-white/65"
            >
              <span class="max-w-48 truncate">{status}</span>
              <span class="font-mono">{progress}%</span>
            </div>
            <div
              class="h-2 overflow-hidden rounded-full border border-white/10 bg-black/30 p-px"
              role="progressbar"
              aria-label="Minecraft launch progress"
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow={progress}
            >
              <div
                class="h-full rounded-full bg-gradient-to-r from-accent-600 via-accent-500 to-accent-400 transition-[width] duration-700 ease-out"
                style={"width: " + progress + "%"}
              ></div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <div class="mc-panel grid grid-cols-3 divide-x divide-white/[0.07] overflow-visible">
    <div class="flex min-w-0 items-center justify-center gap-3 px-5 py-4">
      <span class="grid size-8 shrink-0 place-items-center text-accent-400">
        <Icon name="instances" size={22} />
      </span>
      <div class="min-w-0">
        <div class="text-[8px] font-bold uppercase tracking-[.14em] text-[var(--color-brand-sky)]">
          Selected profile
        </div>
        <h2 class="mt-1 truncate text-sm font-bold text-[var(--color-text-primary)]">
          {instance.name}
        </h2>
        <p class="mt-0.5 truncate text-[9px] text-[var(--color-text-muted)]">
          Minecraft {instance.minecraftVersion}{instance.modpack
            ? " · " + instance.modpack.name
            : ""}
        </p>
      </div>
    </div>

    <div class="flex items-center px-5 py-4">
      <CustomSelect
        value={instance.minecraftVersion}
        options={versions.map((version) => ({
          value: version.minecraftVersion,
          label:
            version.minecraftVersion + (version.versionType === "snapshot" ? " · Snapshot" : ""),
        }))}
        onchange={(value) => onversionchange(String(value))}
        disabled={launching || running || Boolean(instance.modpack)}
        ariaLabel="Minecraft version"
        displayLabel="Change version"
        class="h-10 w-full text-[11px] font-bold text-white"
      />
    </div>

    <div class="grid place-items-center px-5 py-4">
      <button
        onclick={onopenmods}
        class="mc-secondary flex h-10 w-full items-center justify-center gap-2 px-4 text-[11px] font-semibold text-white/80"
        ><Icon name="mods" size={12} />Manage mods</button
      >
    </div>
  </div>
</section>

<style>
  .play-split {
    position: relative;
    isolation: isolate;
    background: var(--gradient-brand);
    background-size: 160% 160%;
    transition:
      transform 220ms cubic-bezier(0.2, 0.8, 0.2, 1),
      box-shadow 220ms ease,
      filter 220ms ease;
  }

  .play-split::before {
    position: absolute;
    z-index: 2;
    inset: 0;
    content: "";
    pointer-events: none;
    background: linear-gradient(
      105deg,
      transparent 30%,
      rgba(255, 255, 255, 0.08) 42%,
      rgba(255, 255, 255, 0.42) 50%,
      rgba(255, 255, 255, 0.08) 58%,
      transparent 70%
    );
    transform: translateX(-125%);
  }

  .play-split.is-ready {
    animation: launch-idle 4.5s ease-in-out infinite;
  }

  .play-split.is-ready::before {
    animation: launch-shine 4.2s cubic-bezier(0.16, 1, 0.3, 1) infinite;
  }

  .play-split.is-launching {
    animation: launch-working 1.1s ease-in-out infinite;
  }

  .play-split.is-launching::before {
    animation: launch-shine 1.15s linear infinite;
  }

  .play-split.is-ready:hover {
    transform: translateY(-2px) scale(1.012);
    box-shadow:
      0 14px 34px var(--color-brand-glow),
      0 0 0 1px rgb(59 157 245 / 0.16);
    filter: brightness(1.07);
  }

  .play-split > button {
    position: relative;
    z-index: 3;
  }

  .launch-glyph {
    display: inline-grid;
    place-items: center;
    transition: transform 220ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .launch-main:hover:not(:disabled) .launch-glyph {
    transform: translateX(3px) scale(1.08);
  }

  .launch-glyph.is-loading {
    animation: launch-glyph-loading 850ms ease-in-out infinite;
  }

  .play-split button:hover:not(:disabled),
  .profile-menu button:hover:not(:disabled) {
    transform: none;
  }

  .profile-menu {
    animation: profile-menu-in 180ms cubic-bezier(0.16, 1, 0.3, 1);
    scrollbar-width: thin;
    scrollbar-color: var(--color-border-brand) transparent;
  }

  .server-logo:hover:not(:disabled) {
    border-color: var(--color-border-brand);
    background: var(--color-surface-hover);
    transform: translateX(-3px) scale(1.04);
  }

  @keyframes profile-menu-in {
    from {
      opacity: 0;
      transform: translateY(5px) scale(0.98);
    }
  }

  @keyframes launch-shine {
    0%,
    62% {
      transform: translateX(-125%);
    }
    100% {
      transform: translateX(125%);
    }
  }

  @keyframes launch-idle {
    0%,
    100% {
      background-position: 0% 50%;
      box-shadow: 0 10px 28px var(--color-brand-glow);
    }
    50% {
      background-position: 100% 50%;
      box-shadow: 0 12px 34px rgb(59 157 245 / 0.3);
    }
  }

  @keyframes launch-working {
    0%,
    100% {
      filter: brightness(0.96);
    }
    50% {
      filter: brightness(1.12);
    }
  }

  @keyframes launch-glyph-loading {
    0%,
    100% {
      transform: translateX(-2px);
      opacity: 0.65;
    }
    50% {
      transform: translateX(3px);
      opacity: 1;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .play-split,
    .play-split::before,
    .launch-glyph {
      animation: none;
    }
  }
</style>
