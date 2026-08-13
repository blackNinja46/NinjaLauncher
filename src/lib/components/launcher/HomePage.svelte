<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";
  import PageHeader from "$lib/components/layout/PageHeader.svelte";
  import type { LauncherInstance, SupportedVersion } from "$lib/types";
  import { launchGame, messageFromError } from "$lib/api/tauri";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  let {
    instance,
    versions,
    signedIn,
    onopenmods,
    onversionchange,
  }: {
    instance: LauncherInstance;
    versions: SupportedVersion[];
    signedIn: boolean;
    onopenmods: () => void;
    onversionchange: (version: string) => Promise<void>;
  } = $props();
  let launching = $state(false);
  let status = $state("Everything is ready");

  onMount(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const unlisteners = [
      listen<{ message: string }>("launcher://status", (event) => (status = event.payload.message)),
      listen("launcher://game-started", () => {
        launching = false;
        status = "Minecraft is running";
      }),
      listen<number | null>("launcher://game-exited", (event) => {
        launching = false;
        status =
          event.payload === 0
            ? "Minecraft closed normally"
            : `Minecraft exited with code ${event.payload ?? "unknown"}`;
      }),
    ];
    return () => {
      Promise.all(unlisteners).then((items) => items.forEach((unlisten) => unlisten()));
    };
  });
  async function play() {
    launching = true;
    status = signedIn ? "Preparing instance…" : "Preparing demo instance…";
    try {
      await launchGame(instance.id, !signedIn);
    } catch (error) {
      launching = false;
      status = messageFromError(error);
    }
  }
</script>

<PageHeader title="Ready to build?" description="Choose your world, gear up, and jump in." />
<section class="mc-panel overflow-hidden">
  <div class="relative min-h-[320px] overflow-hidden border-b border-cyan-300/[0.12] p-8">
    <div class="voxel-pattern absolute inset-0 opacity-90"></div>
    <div class="absolute -right-20 -top-24 size-80 rounded-full bg-cyan-400/[0.08] blur-3xl"></div>
    <div class="relative max-w-[650px]">
      <img
        src="/brand/ninja-launcher-wordmark.png"
        alt="Ninja Launcher"
        class="mb-8 h-auto w-[570px] max-w-full drop-shadow-[0_10px_18px_rgba(0,5,35,.55)]"
      />
      <div
        class="mb-4 inline-flex items-center gap-2 border border-cyan-300/[0.18] bg-blue-950/55 px-2.5 py-1 text-[9px] font-bold uppercase tracking-[.16em] text-cyan-100/70"
      >
        <span class="size-2 bg-emerald-400 shadow-[0_0_8px_#34d399]"></span>Selected instance
      </div>
      <h2 class="block-title text-[28px] uppercase text-white">{instance.name}</h2>
      <div class="mt-4 flex items-center gap-2.5">
        <label
          class="text-[10px] font-semibold uppercase tracking-wider text-[#71849b]"
          for="home-version">Version</label
        >
        <select
          id="home-version"
          value={instance.minecraftVersion}
          onchange={(event) => onversionchange(event.currentTarget.value)}
          disabled={launching}
          class="mc-field px-3 py-2 text-xs font-bold text-blue-50 outline-none disabled:opacity-50"
        >
          {#each versions as version}<option value={version.minecraftVersion}
              >{version.minecraftVersion}</option
            >{/each}
        </select>
        <span class="mc-field px-3 py-2 text-xs text-[#9bc9ed]"
          >Fabric {instance.loaderVersion}</span
        >
      </div>
      <div class="mt-7 flex items-center gap-3">
        <button
          onclick={play}
          disabled={launching}
          class="mc-primary flex min-w-[190px] items-center justify-center gap-2 px-8 py-3.5 text-sm font-extrabold disabled:opacity-70"
          ><Icon name="play" size={17} />{launching
            ? "PREPARING…"
            : signedIn
              ? "PLAY"
              : "PLAY DEMO"}</button
        >
        <button
          onclick={onopenmods}
          class="mc-secondary px-5 py-3.5 text-sm font-bold text-[#ccecff]">Manage mods</button
        >
      </div>
      {#if !signedIn}<p class="mt-3 text-[11px] text-[#8094ad]">
          Demo mode · Local test worlds only · Online multiplayer unavailable
        </p>{/if}
    </div>
  </div>
  <div class="grid grid-cols-3 divide-x divide-cyan-300/[0.1] bg-[#051126]/70">
    <div class="p-5">
      <div class="text-[10px] font-semibold uppercase tracking-wider text-[#6f7480]">Minecraft</div>
      <div class="mt-1.5 text-sm font-medium">{instance.minecraftVersion}</div>
    </div>
    <div class="p-5">
      <div class="text-[10px] font-semibold uppercase tracking-wider text-[#6f7480]">
        Mod loader
      </div>
      <div class="mt-1.5 flex items-center gap-2 text-sm font-medium">
        <span class="size-2 rounded-sm bg-[#d8a33c]"></span>Fabric
      </div>
    </div>
    <div class="p-5">
      <div class="text-[10px] font-semibold uppercase tracking-wider text-[#6f7480]">
        Enabled mods
      </div>
      <div class="mt-1.5 text-sm font-medium">
        {instance.mods.filter((mod) => mod.enabled).length} mods
      </div>
    </div>
  </div>
</section>
<div class="mc-panel mt-4 flex items-center gap-2 px-4 py-3 text-xs text-[#86a8c7]">
  <span class="grid size-5 place-items-center bg-emerald-400/10 text-emerald-400"
    ><Icon name="check" size={12} strokeWidth={2.5} /></span
  >{status}<span class="ml-auto text-[#4f718f]">Last checked just now</span>
</div>
