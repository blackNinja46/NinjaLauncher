<script lang="ts">
  import { onMount } from "svelte";
  import { getServerIcon } from "$lib/api/tauri";
  import NameGradientDialog from "$lib/components/account/NameGradientDialog.svelte";
  import RankBadge from "$lib/components/account/RankBadge.svelte";
  import PlayerSkin from "$lib/components/account/PlayerSkin.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import VersionPicker from "$lib/components/launcher/VersionPicker.svelte";
  import HalloweenScene from "$lib/components/launcher/HalloweenScene.svelte";
  import {
    hasNinjaPlus,
    isNameGradientUnlocked,
    nameGradient,
    rankLogoColor,
    type NameGradient,
  } from "$lib/nameGradients";
  import type { Account, LauncherInstance, PlayerRanks, SupportedVersion } from "$lib/types";

  let {
    instance,
    instances,
    versions,
    signedIn,
    account,
    networkRanks,
    halloweenTheme,
    halloweenSoundsEnabled,
    animationsEnabled,
    launching,
    running,
    stopping,
    progress,
    status,
    maintenanceActive,
    maintenanceMessage,
    maintenanceEndsAt,
    versionAvailable,
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
    halloweenTheme: boolean;
    halloweenSoundsEnabled: boolean;
    animationsEnabled: boolean;
    launching: boolean;
    running: boolean;
    stopping: boolean;
    progress: number | null;
    status: string;
    maintenanceActive: boolean;
    maintenanceMessage: string;
    maintenanceEndsAt: string | null;
    versionAvailable: boolean;
    onplay: () => Promise<void>;
    onconnect: (serverAddress: string) => Promise<void>;
    onstop: () => Promise<void>;
    onopenmods: () => void;
    onversionchange: (version: string) => Promise<void>;
    onprofilechange: (id: string) => void;
  } = $props();

  let profileMenu: HTMLDivElement;
  let profileMenuOpen = $state(false);
  let gradientDialogOpen = $state(false);
  let selectedGradientId = $state("none");
  const featuredServers = [
    { name: "blackNinja.Live", address: "blackninja.live" },
    { name: "NinjaSMP.net", address: "ninjasmp.net" },
  ];
  let serverIcons = $state<Record<string, string>>({});
  let primaryRank = $derived(
    networkRanks?.ranks.find((rank) => rank.primary) ?? networkRanks?.ranks[0] ?? null,
  );
  let selectedGradient = $derived(nameGradient(selectedGradientId));
  let playerRankLogoColor = $derived(rankLogoColor(networkRanks));
  let playerHasNinjaPlus = $derived(hasNinjaPlus(networkRanks));

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

  function playHalloweenLaunchSound() {
    if (!halloweenTheme || !halloweenSoundsEnabled || typeof AudioContext === "undefined") return;
    const context = new AudioContext();
    const master = context.createGain();
    master.gain.setValueAtTime(0.0001, context.currentTime);
    master.gain.exponentialRampToValueAtTime(0.16, context.currentTime + 0.035);
    master.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + 1.45);
    master.connect(context.destination);

    const notes = [196, 155.56, 116.54];
    notes.forEach((frequency, index) => {
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      const start = context.currentTime + index * 0.17;
      oscillator.type = index === 2 ? "triangle" : "sine";
      oscillator.frequency.setValueAtTime(frequency, start);
      oscillator.frequency.exponentialRampToValueAtTime(frequency * 0.92, start + 0.55);
      gain.gain.setValueAtTime(0.0001, start);
      gain.gain.exponentialRampToValueAtTime(0.42 - index * 0.07, start + 0.025);
      gain.gain.exponentialRampToValueAtTime(0.0001, start + 0.7);
      oscillator.connect(gain).connect(master);
      oscillator.start(start);
      oscillator.stop(start + 0.72);
    });

    const noiseLength = Math.floor(context.sampleRate * 1.1);
    const noiseBuffer = context.createBuffer(1, noiseLength, context.sampleRate);
    const noise = noiseBuffer.getChannelData(0);
    for (let index = 0; index < noise.length; index += 1) {
      noise[index] = (Math.random() * 2 - 1) * (1 - index / noise.length);
    }
    const wind = context.createBufferSource();
    const windFilter = context.createBiquadFilter();
    const windGain = context.createGain();
    wind.buffer = noiseBuffer;
    windFilter.type = "lowpass";
    windFilter.frequency.value = 520;
    windGain.gain.value = 0.16;
    wind.connect(windFilter).connect(windGain).connect(master);
    wind.start();
    wind.stop(context.currentTime + 1.1);
    window.setTimeout(() => void context.close(), 1_700);
  }

  async function launchFromButton() {
    playHalloweenLaunchSound();
    await onplay();
  }

  function selectNameGradient(gradient: NameGradient) {
    if (!account || !isNameGradientUnlocked(gradient, networkRanks)) return;
    selectedGradientId = gradient.id;
    localStorage.setItem(`ninja-launcher:name-gradient:${account.uuid}`, gradient.id);
    window.dispatchEvent(
      new CustomEvent("ninja-name-gradient-changed", {
        detail: { uuid: account.uuid, gradientId: gradient.id },
      }),
    );
    gradientDialogOpen = false;
  }

  function formatMaintenanceTime(value: string | null): string {
    if (!value) return "Not announced";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "Not announced";
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(date);
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

  $effect(() => {
    if (typeof window === "undefined") return;
    const owner = account?.uuid;
    if (!owner) {
      selectedGradientId = "none";
      return;
    }
    const stored = localStorage.getItem(`ninja-launcher:name-gradient:${owner}`) ?? "none";
    const gradient = nameGradient(stored);
    selectedGradientId = isNameGradientUnlocked(gradient, networkRanks) ? gradient.id : "none";
  });
</script>

<section class:halloween-home={halloweenTheme} class="home-page group">
  <div class="relative h-[clamp(480px,calc(100vh-136px),574px)] px-5 sm:px-7">
    <div class="relative flex h-full flex-col items-center">
      {#if maintenanceActive}
        <div
          class="maintenance-screen absolute inset-0 z-50 flex items-center justify-center px-4 py-5"
          role="alert"
          aria-labelledby="maintenance-title"
        >
          <div
            class="maintenance-card relative w-full max-w-[700px] overflow-hidden rounded-xl px-6 py-6 shadow-[0_28px_90px_rgba(0,0,0,.72)] backdrop-blur-2xl sm:px-8"
          >
            <div
              class="maintenance-glow pointer-events-none absolute left-1/2 top-0 h-36 w-72 -translate-x-1/2 rounded-full bg-red-500/20 blur-3xl"
            ></div>

            <div class="relative flex flex-col items-center text-center">
              <div class="maintenance-lock text-red-300 shadow-[rgba(248,70,70,.22)]">
                <Icon name="lock" size={32} />
              </div>

              <div
                class="mt-4 inline-flex items-center gap-2 rounded-full border border-red-300/15 bg-red-400/[.08] px-3 py-1 text-[9px] font-black uppercase tracking-[.18em] text-red-300"
              >
                <span class="size-1.5 rounded-full bg-red-400"></span>
                System maintenance
              </div>
              <h2
                id="maintenance-title"
                class="mt-3 text-[22px] font-black tracking-[-.025em] text-white"
              >
                Minecraft launch is temporarily disabled
              </h2>
              <p class="mt-2 max-w-[540px] text-[12px] leading-5 text-[var(--color-text-muted)]">
                {maintenanceMessage ||
                  "The Ninja Launcher service is currently being maintained. Minecraft cannot be started until maintenance is complete."}
              </p>
            </div>

            <div class="relative mt-5 grid grid-cols-3 gap-2.5">
              <div
                class="maintenance-status rounded-[10px] border border-white/[.07] bg-white/[.035] px-3 py-3"
              >
                <span class="flex items-center gap-2 text-red-300"
                  ><Icon name="lock" size={12} /><span
                    class="text-[9px] font-black uppercase tracking-[.12em]">Game launch</span
                  ></span
                >
                <p class="mt-1.5 text-[11px] font-bold text-white">Disabled</p>
              </div>
              <div
                class="maintenance-status rounded-[10px] border border-white/[.07] bg-white/[.035] px-3 py-3"
              >
                <span class="flex items-center gap-2 text-[var(--color-primary)]"
                  ><Icon name="check" size={12} /><span
                    class="text-[9px] font-black uppercase tracking-[.12em]">Your profiles</span
                  ></span
                >
                <p class="mt-1.5 text-[11px] font-bold text-white">Remain safe</p>
              </div>
              <div
                class="maintenance-status rounded-[10px] border border-white/[.07] bg-white/[.035] px-3 py-3"
              >
                <span class="flex items-center gap-2 text-[var(--color-primary)]"
                  ><Icon name="clock" size={12} /><span
                    class="text-[9px] font-black uppercase tracking-[.12em]">Expected end</span
                  ></span
                >
                <p
                  class="mt-1.5 truncate text-[11px] font-bold text-white"
                  title={formatMaintenanceTime(maintenanceEndsAt)}
                >
                  {formatMaintenanceTime(maintenanceEndsAt)}
                </p>
              </div>
            </div>

            <div
              class="relative mt-4 flex items-start gap-3 rounded-[10px] border border-white/[.07] bg-black/15 px-4 py-3 text-left"
            >
              <div class="flex-inline flex shrink-0 gap-2 items-center justify-center">
                <span class="mt-0.5 text-[var(--color-primary)]"
                  ><Icon name="info" size={16} /></span
                >
                <p class="text-[10px] text-center leading-4 text-[var(--color-text-muted)]">
                  The launcher checks the service status automatically. The lock disappears as soon
                  as maintenance ends—no restart is required.
                </p>
              </div>
            </div>
          </div>
        </div>
      {:else if !versionAvailable}
        <div
          class="absolute rounded-xl left-1/2 top-0.5 z-50 w-[min(92%,560px)] -translate-x-1/2 border border-amber-300/30 bg-amber-400/10 px-4 py-3 text-center shadow-[0_12px_36px_rgba(0,0,0,.35)] backdrop-blur-xl"
          role="alert"
        >
          <p class="text-md font-bold uppercase tracking-[.16em] text-amber-200">
            Version unavailable
          </p>
          <p class="mt-1 text-sm text-amber-50/80">
            Minecraft {instance.minecraftVersion} was disabled in the dashboard. Choose another version
            to continue.
          </p>
        </div>
      {/if}
      <div
        class="home-identity pointer-events-none absolute left-1/2 top-10 z-30 flex -translate-x-1/2 flex-col items-center gap-1"
      >
        {#if primaryRank}<RankBadge rank={primaryRank} compact />{/if}
        <div class="pointer-events-auto flex items-center gap-2">
          <span class="relative grid size-7 shrink-0 place-items-center" aria-hidden="true">
            <span
              class="rank-launcher-logo absolute inset-0"
              style={`--rank-logo-color: ${playerRankLogoColor}`}
            ></span>
            {#if playerHasNinjaPlus}
              <span
                class="absolute right-[1px] bottom-[2px] z-10 text-[14px] font-black leading-none text-white drop-shadow-[0_1px_2px_rgba(0,0,0,.95)]"
                >+</span
              >
            {/if}
          </span>
          <h1
            class:home-name-gradient={selectedGradient.id !== "none"}
            class="whitespace-nowrap text-[26px] font-extrabold tracking-[-.035em] text-white drop-shadow-[0_3px_12px_rgba(0,0,0,.65)]"
            style={selectedGradient.id === "none"
              ? undefined
              : `--name-gradient-colors: ${selectedGradient.colors.join(", ")};`}
          >
            {account?.username ?? "Guest"}
          </h1>
          {#if account}
            <button
              type="button"
              onclick={() => (gradientDialogOpen = true)}
              class="name-edit-button grid size-7 place-items-center text-white/55 transition hover:text-white"
              aria-label="Choose an animated name gradient"
              title="Customize name gradient"
            >
              <Icon name="edit" size={11} />
            </button>
          {/if}
        </div>
      </div>

      <div
        class="home-player-stage relative min-h-0 w-[clamp(420px,52%,560px)] flex-1 translate-y-3 pt-20"
      >
        {#if halloweenTheme}
          <div
            class="halloween-diorama pointer-events-none absolute bottom-0 top-8"
            aria-hidden="true"
          >
            <HalloweenScene animated={animationsEnabled} />
          </div>
        {/if}
        <div class="relative z-10 size-full">
          <PlayerSkin {account} {launching} rank={primaryRank} showNameTag={false} />
        </div>
      </div>

      <aside
        class="home-servers absolute right-0 top-1/2 z-30 flex -translate-y-1/2 flex-col gap-2.5"
        aria-label="Featured servers"
      >
        {#each featuredServers as server}
          <button
            type="button"
            onclick={() => onconnect(server.address)}
            disabled={launching || running || !signedIn || maintenanceActive || !versionAvailable}
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

      <div
        bind:this={profileMenu}
        class="home-play relative z-40 w-72 shrink-0 -translate-y-8 pb-2"
      >
        <div
          class="play-split {launching
            ? 'is-launching'
            : running
              ? stopping
                ? 'is-running is-stopping'
                : 'is-running'
              : maintenanceActive
                ? 'is-locked'
                : 'is-ready'} flex overflow-hidden rounded-[8px] border text-lg font-bold text-white"
        >
          {#if running}
            <button
              onclick={onstop}
              disabled={stopping}
              class="stop-main flex min-w-0 flex-1 items-center justify-center gap-2 px-6 py-3 text-sm font-extrabold tracking-[-.01em]"
            >
              <span class="launch-glyph is-stop {stopping ? 'is-loading' : ''}">
                <Icon name="stop" size={11} />
              </span>{stopping ? "Stopping…" : "Stop game"}
            </button>
          {:else}
            <button
              onclick={launchFromButton}
              disabled={launching || maintenanceActive || !versionAvailable}
              class="launch-main flex min-w-0 flex-1 items-center justify-center gap-2 px-6 py-3"
            >
              <span class="launch-glyph {launching ? 'is-loading' : ''}">
                <Icon name={maintenanceActive ? "lock" : "play"} size={15} />
              </span>{maintenanceActive
                ? "Maintenance"
                : !versionAvailable
                  ? "Version unavailable"
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
            class="profile-trigger grid w-14 shrink-0 place-items-center border-l border-white/30 bg-black/10 hover:bg-white/10 disabled:cursor-not-allowed disabled:opacity-45"
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
                  {#if profile.loader === "vanilla"}<Icon name="vanilla" size={13} />{:else}<img
                      src="/loaders/fabric.png"
                      alt=""
                      class="size-5 object-contain"
                    />{/if}
                </span>
                <span class="min-w-0 flex-1">
                  <span class="block truncate text-[10px] font-bold">{profile.name}</span>
                  <span class="block text-[8px] text-white/40"
                    >Minecraft {profile.minecraftVersion} · {profile.loader === "vanilla"
                      ? "Vanilla"
                      : "Fabric"}</span
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

  <div
    class="home-profile-bar mc-panel -mt-4 grid grid-cols-3 divide-x divide-white/[0.07] overflow-visible"
  >
    <div class="flex min-w-0 items-center justify-center gap-3 px-5 py-4">
      <span
        class="grid size-9 shrink-0 place-items-center overflow-hidden rounded-lg border border-white/10 bg-black/20 text-accent-400 shadow-[0_6px_16px_rgba(0,0,0,.2)]"
      >
        {#if instance.modpack?.iconUrl}<img
            src={instance.modpack.iconUrl}
            alt={`${instance.modpack.name} modpack`}
            class="size-full object-cover"
          />{:else if instance.modpack}<Icon
            name="modpacks"
            size={19}
          />{:else if instance.loader === "vanilla"}<Icon name="vanilla" size={19} />{:else}<img
            src="/loaders/fabric.png"
            alt=""
            class="size-8 object-contain"
          />{/if}
      </span>
      <div class="min-w-0">
        <div class="text-[8px] font-bold uppercase tracking-[.14em] text-[var(--color-brand-sky)]">
          Selected profile
        </div>
        <h2 class="mt-1 truncate text-sm font-bold text-[var(--color-text-primary)]">
          {instance.name}
        </h2>
        <p class="mt-0.5 truncate text-[9px] text-[var(--color-text-muted)]">
          Minecraft {instance.minecraftVersion} · {instance.loader === "vanilla"
            ? "Vanilla"
            : "Fabric"}{instance.modpack ? " · " + instance.modpack.name : ""}
        </p>
      </div>
    </div>

    <div class="flex items-center px-5 py-4">
      <VersionPicker
        value={instance.minecraftVersion}
        {versions}
        loader={instance.loader}
        onchange={onversionchange}
        disabled={launching || running || Boolean(instance.modpack)}
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

{#if gradientDialogOpen && account}
  <NameGradientDialog
    username={account.username}
    ranks={networkRanks}
    selected={selectedGradientId}
    onselect={selectNameGradient}
    onclose={() => (gradientDialogOpen = false)}
  />
{/if}

<style>
  .home-name-gradient {
    color: transparent;
    background-image: linear-gradient(90deg, var(--name-gradient-colors));
    background-size: 160% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    filter: saturate(1.25) brightness(1.1)
      drop-shadow(0 2px 7px color-mix(in srgb, var(--name-gradient-glow) 36%, transparent));
    animation: home-name-gradient-flow 1.6s ease-in-out infinite alternate;
  }

  .rank-launcher-logo {
    background-color: var(--rank-logo-color);
    filter: drop-shadow(0 3px 9px color-mix(in srgb, var(--rank-logo-color) 35%, transparent));
    -webkit-mask: url("/brand/ninja-launcher-app-icon.png") center / contain no-repeat;
    mask: url("/brand/ninja-launcher-app-icon.png") center / contain no-repeat;
  }

  .name-edit-button :global(svg) {
    transition: transform 180ms ease;
  }

  .name-edit-button:hover :global(svg) {
    transform: rotate(-8deg) scale(1.08);
  }

  .halloween-home .home-player-stage {
    filter: drop-shadow(0 18px 34px rgb(0 0 0 / 0.28));
  }

  .halloween-diorama {
    z-index: 0;
    left: 50%;
    width: min(920px, calc(100vw - 276px));
    overflow: visible;
    transform: translateX(-50%);
    perspective: 500px;
  }

  .halloween-scene-moon {
    position: absolute;
    top: 15%;
    right: 17%;
    color: #ff9c3d;
    filter: drop-shadow(0 0 20px rgb(255 107 24 / 0.52));
    opacity: 0.78;
  }

  .halloween-scene-crow {
    position: absolute;
    z-index: 2;
    top: 30%;
    right: 27%;
    color: #16091d;
    transform: rotate(-10deg);
    animation: halloween-crow 5.6s ease-in-out infinite;
  }

  .halloween-scene-tree {
    position: absolute;
    z-index: 2;
    bottom: 13%;
    color: #120817;
    filter: drop-shadow(0 8px 8px rgb(0 0 0 / 0.55));
    transform-origin: bottom;
  }

  .halloween-scene-tree.is-left {
    left: 8%;
    transform: rotate(-8deg) skewY(-3deg);
  }

  .halloween-scene-tree.is-right {
    right: 9%;
    transform: rotate(7deg) scaleX(-1);
  }

  .halloween-ground {
    position: absolute;
    z-index: 1;
    right: 7%;
    bottom: 3%;
    left: 7%;
    height: 34%;
    border-radius: 50%;
    background:
      radial-gradient(ellipse at center, rgb(98 45 128 / 0.44), rgb(20 9 27 / 0.9) 66%), #09050c;
    box-shadow:
      inset 0 12px 25px rgb(172 78 236 / 0.09),
      0 16px 30px rgb(0 0 0 / 0.48);
    transform: rotateX(62deg) translateZ(-35px);
  }

  .halloween-grave {
    position: absolute;
    z-index: 3;
    display: grid;
    width: 34px;
    height: 48px;
    place-items: center;
    border: 1px solid rgb(194 134 229 / 0.2);
    border-radius: 16px 16px 3px 3px;
    background: linear-gradient(145deg, #35213e, #160d1c);
    box-shadow:
      inset 3px 2px 5px rgb(255 255 255 / 0.04),
      7px 9px 12px rgb(0 0 0 / 0.42);
    color: rgb(214 182 226 / 0.35);
    font: 700 7px/1 monospace;
    transform-origin: bottom;
  }

  .halloween-grave.is-left {
    bottom: 13%;
    left: 25%;
    transform: rotate(-9deg) perspective(100px) rotateY(12deg);
  }

  .halloween-grave.is-right {
    right: 24%;
    bottom: 15%;
    transform: rotate(8deg) scale(0.86) perspective(100px) rotateY(-14deg);
  }

  .halloween-pumpkin {
    position: absolute;
    z-index: 5;
    width: 43px;
    height: 34px;
    border-radius: 48%;
    background:
      radial-gradient(
        ellipse at 28% 50%,
        transparent 38%,
        rgb(92 27 5 / 0.26) 42% 46%,
        transparent 50%
      ),
      radial-gradient(
        ellipse at 70% 50%,
        transparent 38%,
        rgb(92 27 5 / 0.26) 42% 46%,
        transparent 50%
      ),
      linear-gradient(145deg, #ff9b32, #c7480e);
    box-shadow:
      inset 0 3px 4px rgb(255 218 139 / 0.3),
      0 0 18px rgb(255 93 10 / 0.3),
      5px 8px 10px rgb(0 0 0 / 0.38);
  }

  .halloween-pumpkin::before,
  .halloween-pumpkin::after {
    position: absolute;
    top: 13px;
    width: 7px;
    height: 6px;
    content: "";
    background: #2c0e31;
    clip-path: polygon(50% 100%, 0 0, 100% 0);
  }

  .halloween-pumpkin::before {
    left: 10px;
  }

  .halloween-pumpkin::after {
    right: 10px;
  }

  .halloween-pumpkin.is-left {
    bottom: 10%;
    left: 16%;
    transform: rotate(-6deg) scale(0.9);
  }

  .halloween-pumpkin.is-right {
    right: 14%;
    bottom: 8%;
    transform: rotate(5deg) scale(0.72);
  }

  .halloween-fog {
    position: absolute;
    z-index: 6;
    right: -8%;
    bottom: 4%;
    left: -8%;
    height: 48px;
    border-radius: 50%;
    background: rgb(178 108 229 / 0.11);
    filter: blur(13px);
    animation: halloween-fog-drift 8s ease-in-out infinite alternate;
  }

  .halloween-fog.is-two {
    bottom: 14%;
    opacity: 0.45;
    animation-delay: -4s;
    animation-direction: alternate-reverse;
  }

  .halloween-home .play-split.is-ready {
    border-color: rgb(255 190 122 / 0.72);
    border-bottom-color: rgb(111 39 145 / 0.94);
    background: linear-gradient(180deg, #ff9e35 0%, #dd5c14 52%, #7230a8 130%);
    box-shadow:
      inset 0 1px 0 rgb(255 239 208 / 0.55),
      inset 0 -4px 0 rgb(80 25 103 / 0.48),
      0 4px 0 rgb(55 17 71 / 0.92),
      0 13px 32px rgb(226 82 14 / 0.28),
      0 0 24px rgb(133 54 206 / 0.18);
  }

  .maintenance-screen {
    isolation: isolate;
    animation: maintenance-screen-in 360ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .maintenance-screen::before {
    position: absolute;
    z-index: -1;
    inset: -72px -96px;
    content: "";
    pointer-events: none;
    background:
      radial-gradient(circle at 50% 35%, rgb(239 68 68 / 0.16), transparent 38%),
      radial-gradient(
        ellipse at center,
        rgb(2 7 15 / 0.86) 0%,
        rgb(2 7 15 / 0.7) 58%,
        transparent 100%
      );
    backdrop-filter: blur(12px) saturate(0.68);
    -webkit-backdrop-filter: blur(12px) saturate(0.68);
    -webkit-mask-image: radial-gradient(
      ellipse at center,
      #000 48%,
      rgb(0 0 0 / 0.92) 64%,
      transparent 100%
    );
    mask-image: radial-gradient(
      ellipse at center,
      #000 48%,
      rgb(0 0 0 / 0.92) 64%,
      transparent 100%
    );
  }

  .maintenance-card {
    animation: maintenance-card-in 520ms cubic-bezier(0.16, 1, 0.3, 1);
  }

  .maintenance-lock {
    animation: maintenance-lock-pulse 2.8s ease-in-out infinite;
  }

  .maintenance-status {
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.025);
  }

  .play-split {
    position: relative;
    isolation: isolate;
    border-color: rgb(176 232 255 / 0.78);
    border-bottom-color: rgb(35 110 189 / 0.88);
    background: linear-gradient(
      180deg,
      rgb(100 194 255 / 0.98) 0%,
      rgb(38 145 237 / 0.98) 48%,
      rgb(20 102 203 / 0.99) 100%
    );
    background-size: 160% 160%;
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.68),
      inset 0 -4px 0 rgb(5 63 143 / 0.48),
      0 4px 0 rgb(4 46 106 / 0.92),
      0 13px 30px rgb(10 101 202 / 0.3),
      0 0 0 1px rgb(0 15 46 / 0.35);
    text-shadow: 0 1px 0 rgb(0 34 91 / 0.46);
    transition:
      transform 160ms cubic-bezier(0.2, 0.8, 0.2, 1),
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
    border-color: rgb(233 213 255 / 0.76);
    border-bottom-color: rgb(107 33 168 / 0.94);
    background: linear-gradient(
      180deg,
      rgb(192 132 252 / 0.99) 0%,
      rgb(147 51 234 / 0.99) 48%,
      rgb(107 33 168 / 1) 100%
    );
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.55),
      inset 0 -4px 0 rgb(73 20 120 / 0.5),
      0 4px 0 rgb(59 7 100 / 0.94),
      0 13px 32px rgb(126 34 206 / 0.34),
      0 0 0 1px rgb(46 4 77 / 0.4);
    text-shadow: 0 1px 0 rgb(59 7 100 / 0.55);
    animation: launch-working 1.1s ease-in-out infinite;
  }

  .play-split.is-launching::before {
    animation: launch-shine 1.15s linear infinite;
  }

  .play-split.is-locked {
    border-color: rgb(248 113 113 / 0.35);
    background: linear-gradient(135deg, rgb(65 26 35 / 0.96), rgb(38 22 32 / 0.98));
    box-shadow: 0 10px 28px rgb(0 0 0 / 0.32);
  }

  .play-split.is-running {
    border-color: rgb(254 202 202 / 0.72);
    border-bottom-color: rgb(153 27 27 / 0.94);
    background: linear-gradient(
      180deg,
      rgb(248 113 113 / 0.99) 0%,
      rgb(220 38 38 / 0.99) 48%,
      rgb(153 27 27 / 1) 100%
    );
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.5),
      inset 0 -4px 0 rgb(96 14 14 / 0.5),
      0 4px 0 rgb(82 12 12 / 0.96),
      0 13px 30px rgb(185 28 28 / 0.3),
      0 0 0 1px rgb(50 4 4 / 0.42);
    text-shadow: 0 1px 0 rgb(86 8 8 / 0.55);
  }

  .play-split.is-running::before {
    animation: stop-shine 4.8s cubic-bezier(0.16, 1, 0.3, 1) infinite;
  }

  .play-split.is-stopping {
    animation: stop-working 900ms ease-in-out infinite;
  }

  .play-split.is-ready:hover {
    animation: none;
    transform: translateY(-2px) scale(1.012);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.72),
      inset 0 -4px 0 rgb(5 63 143 / 0.5),
      0 5px 0 rgb(4 46 106 / 0.94),
      0 17px 36px rgb(10 101 202 / 0.4),
      0 0 0 1px rgb(129 220 255 / 0.24);
    filter: brightness(1.07);
  }

  .play-split.is-running:hover {
    transform: translateY(-1px);
    filter: brightness(1.06);
  }

  .play-split:has(.launch-main:active),
  .play-split:has(.stop-main:active) {
    transform: translateY(3px) scale(0.995);
    box-shadow:
      inset 0 2px 6px rgb(0 0 0 / 0.2),
      inset 0 -2px 0 rgb(0 0 0 / 0.24),
      0 1px 0 rgb(0 20 56 / 0.9),
      0 6px 15px rgb(0 0 0 / 0.22);
    transition-duration: 80ms;
  }

  .play-split > button {
    position: relative;
    z-index: 3;
  }

  .launch-glyph {
    display: inline-grid;
    width: 26px;
    height: 26px;
    place-items: center;
    transition: transform 220ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .profile-trigger {
    box-shadow: inset 1px 0 0 rgb(0 0 0 / 0.12);
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

  @keyframes halloween-crow {
    0%,
    100% {
      transform: translate(0, 0) rotate(-10deg);
    }
    50% {
      transform: translate(-8px, -4px) rotate(-3deg);
    }
  }

  @keyframes halloween-fog-drift {
    from {
      transform: translateX(-4%) scaleX(0.92);
    }
    to {
      transform: translateX(5%) scaleX(1.06);
    }
  }

  @keyframes home-name-gradient-flow {
    from {
      background-position: 0% 50%;
    }
    to {
      background-position: 100% 50%;
    }
  }

  :global(.animations-disabled) .home-name-gradient {
    animation: none;
    background-position: 50% 50%;
  }

  :global(.animations-disabled) .halloween-scene-crow,
  :global(.animations-disabled) .halloween-fog {
    animation: none;
  }

  @keyframes maintenance-screen-in {
    from {
      opacity: 0;
    }
  }

  @keyframes maintenance-card-in {
    from {
      opacity: 0;
      transform: translateY(18px) scale(0.975);
    }
  }

  @keyframes maintenance-lock-pulse {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-3px);
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
      box-shadow:
        inset 0 1px 0 rgb(255 255 255 / 0.68),
        inset 0 -4px 0 rgb(5 63 143 / 0.48),
        0 4px 0 rgb(4 46 106 / 0.92),
        0 13px 30px rgb(10 101 202 / 0.3),
        0 0 0 1px rgb(0 15 46 / 0.35);
    }
    50% {
      background-position: 100% 50%;
      box-shadow:
        inset 0 1px 0 rgb(255 255 255 / 0.72),
        inset 0 -4px 0 rgb(5 63 143 / 0.5),
        0 4px 0 rgb(4 46 106 / 0.94),
        0 16px 36px rgb(59 157 245 / 0.38),
        0 0 0 1px rgb(129 220 255 / 0.2);
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

  @keyframes stop-shine {
    0%,
    68% {
      transform: translateX(-125%);
    }
    100% {
      transform: translateX(125%);
    }
  }

  @keyframes stop-working {
    0%,
    100% {
      filter: brightness(0.92);
    }
    50% {
      filter: brightness(1.08);
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
    .maintenance-screen,
    .maintenance-card,
    .maintenance-lock,
    .play-split,
    .play-split::before,
    .launch-glyph,
    .home-name-gradient,
    .halloween-scene-crow,
    .halloween-fog {
      animation: none;
    }
  }
</style>
