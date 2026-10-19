<script lang="ts">
  import { onMount } from "svelte";
  import type { SkinViewer } from "skinview3d";
  import { appearances } from "./appearance.svelte";
  import type { SkinModel } from "$lib/api/skins";
  import { applyWardrobePose } from "./skinAnimation";
  import { applySkinPose } from "./skinAnimation";
  import { getAccountCape } from "$lib/api/tauri";
  import RankBadge from "$lib/components/account/RankBadge.svelte";
  import type { Account, NetworkRank } from "$lib/types";

  let {
    account,
    rank,
    showNameTag = true,
    launching = false,
    texture,
    model,
    previewCape,
    animation = "idle",
    compact = false,
    interactive = true,
    showEffects = true,
  }: {
    account: Account | null;
    rank: NetworkRank | null;
    showNameTag?: boolean;
    launching?: boolean;
    texture?: string;
    model?: SkinModel;
    previewCape?: string | null;
    animation?: "idle" | "wardrobe" | "still";
    compact?: boolean;
    interactive?: boolean;
    showEffects?: boolean;
  } = $props();
  let launchCue = 0;
  $effect(() => {
    if (launching) launchCue += 1;
  });
  let stage: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let viewer = $state.raw<SkinViewer>();
  let failed = $state(false);
  let capeUrl = $state<string | null>(null);
  let loadedSkin = "";
  let loadedCape: string | null | undefined;
  const appearance = $derived(account ? appearances[account.uuid] : undefined);
  const skinUrl = $derived(
    texture ??
      appearance?.texture ??
      `https://mc-heads.net/skin/${encodeURIComponent(account?.uuid ?? "MHF_Steve")}`,
  );
  const selectedModel = $derived(model ?? (texture ? undefined : appearance?.model));
  const selectedCape = $derived(
    previewCape !== undefined ? previewCape : appearance ? appearance.cape : capeUrl,
  );

  async function loadSkin(url: string) {
    if (!viewer || loadedSkin === url) return;
    const target = viewer;
    loadedSkin = url;
    failed = false;
    try {
      const image = new Image();
      image.crossOrigin = "anonymous";
      image.src = url;
      await image.decode();
      if (viewer !== target || target.disposed || loadedSkin !== url) return;
      target.loadSkin(image);
      if (selectedModel)
        target.playerObject.skin.modelType = selectedModel === "classic" ? "default" : "slim";
    } catch {
      if (loadedSkin === url) failed = true;
    }
  }

  async function loadCape(url: string | null) {
    if (!viewer || loadedCape === url) return;
    const target = viewer;
    loadedCape = url;
    try {
      if (url) {
        const image = new Image();
        image.crossOrigin = "anonymous";
        image.src = url;
        await image.decode();
        if (viewer !== target || target.disposed || loadedCape !== url) return;
        target.loadCape(image, { backEquipment: "cape" });
      } else target.loadCape(null);
    } catch {
      if (viewer === target && !target.disposed && loadedCape === url) {
        target.loadCape(null);
        loadedCape = null;
      }
    }
  }

  $effect(() => {
    const url = skinUrl;
    if (viewer) void loadSkin(url);
  });

  $effect(() => {
    const uuid = account?.uuid;
    capeUrl = null;
    if (!uuid || previewCape !== undefined || appearance) return;
    let current = true;
    getAccountCape(uuid)
      .then((url) => {
        if (current && account?.uuid === uuid) capeUrl = url;
      })
      .catch(() => {
        if (current && account?.uuid === uuid) capeUrl = null;
      });
    return () => {
      current = false;
    };
  });

  $effect(() => {
    const url = selectedCape;
    if (viewer) void loadCape(url);
  });

  $effect(() => {
    if (viewer && selectedModel)
      viewer.playerObject.skin.modelType = selectedModel === "classic" ? "default" : "slim";
  });

  onMount(() => {
    let disposed = false;
    let observer: ResizeObserver | undefined;
    const preventViewerScroll = (event: WheelEvent) => event.preventDefault();
    if (interactive) canvas.addEventListener("wheel", preventViewerScroll, { passive: false });
    void import("skinview3d").then(({ FunctionAnimation, SkinViewer }) => {
      if (disposed) return;
      viewer = new SkinViewer({
        canvas,
        width: Math.max(180, stage.clientWidth),
        height: Math.max(250, stage.clientHeight),
      });
      viewer.background = null;
      viewer.fov = 50;
      viewer.zoom = compact || animation === "wardrobe" ? 0.76 : 0.86;
      viewer.autoRotate = false;
      viewer.globalLight.intensity = 2.2;
      viewer.cameraLight.intensity = 1.05;
      viewer.cameraLight.position.set(-12, 18, 8);
      viewer.cameraLight.decay = 0;
      viewer.controls.enableRotate = interactive;
      viewer.controls.enabled = interactive;
      viewer.controls.enableZoom = false;
      viewer.controls.enablePan = false;
      viewer.controls.minPolarAngle = Math.PI / 2;
      viewer.controls.maxPolarAngle = Math.PI / 2;
      viewer.controls.rotateSpeed = 0.72;
      viewer.controls.update();
      viewer.playerWrapper.position.y = compact || animation === "wardrobe" ? -1 : -7;
      const motionPreference = window.matchMedia("(prefers-reduced-motion: reduce)");
      let seenLaunchCue = 0;
      let launchStarted = -Infinity;
      viewer.animation = new FunctionAnimation((player, progress) => {
        if (seenLaunchCue !== launchCue) {
          seenLaunchCue = launchCue;
          launchStarted = progress;
        }
        if (animation === "wardrobe") applyWardrobePose(player, progress, motionPreference.matches);
        else
          applySkinPose(
            player,
            progress,
            progress - launchStarted,
            motionPreference.matches || animation === "still",
          );
      });
      viewer.playerObject.rotation.y = -0.22;
      void loadSkin(skinUrl);
      void loadCape(selectedCape);

      observer = new ResizeObserver(([entry]) => {
        if (!viewer || !entry) return;
        viewer.setSize(
          Math.max(180, Math.round(entry.contentRect.width)),
          Math.max(250, Math.round(entry.contentRect.height)),
        );
      });
      observer.observe(stage);
    });
    return () => {
      disposed = true;
      canvas.removeEventListener("wheel", preventViewerScroll);
      observer?.disconnect();
      viewer?.dispose();
      viewer = undefined;
    };
  });
</script>

<div
  bind:this={stage}
  class="player-stage relative flex h-full {compact
    ? 'min-h-[250px]'
    : 'min-h-[300px]'} items-end justify-center overflow-hidden"
  aria-label={interactive ? "Interactive 3D player skin" : "3D player skin"}
>
  {#if showNameTag}
    <div
      class="name-tag pointer-events-none absolute left-1/2 top-4 z-20 flex -translate-x-1/2 flex-col items-center gap-1.5"
    >
      {#if rank}<RankBadge {rank} compact />{/if}
      <span
        class="whitespace-nowrap border border-black/40 bg-black/55 px-2 py-0.5 font-mono text-[11px] font-semibold text-white shadow-[0_2px_8px_rgba(0,0,0,.55)] backdrop-blur-[2px]"
      >
        {account?.username ?? "Guest"}
      </span>
    </div>
  {/if}

  {#if showEffects}<div
      class="scene-pixels pointer-events-none absolute inset-0"
      aria-hidden="true"
    >
      <i></i><i></i><i></i><i></i><i></i><i></i>
    </div>{/if}
  <canvas
    bind:this={canvas}
    class="relative z-10 size-full drop-shadow-[0_16px_18px_rgba(0,0,0,.48)] {interactive
      ? 'cursor-grab touch-none active:cursor-grabbing'
      : 'pointer-events-none'}"
    aria-label={interactive
      ? "Drag left or right to rotate the Minecraft skin"
      : "Minecraft skin render"}
  ></canvas>
  {#if failed}
    <div
      class="pointer-events-none absolute inset-x-4 top-1/2 z-30 -translate-y-1/2 border border-amber-300/15 bg-[#071a36]/90 px-3 py-2 text-center text-[10px] font-semibold text-amber-100/70"
    >
      Skin texture unavailable
    </div>
  {/if}
</div>

<style>
  .player-stage {
    isolation: isolate;
  }

  .scene-pixels i {
    position: absolute;
    width: 3px;
    height: 3px;
    background: rgba(111, 226, 255, 0.62);
    box-shadow: 0 0 8px rgba(59, 157, 245, 0.55);
    animation: pixel-drift 5s ease-in-out infinite;
  }

  .scene-pixels i:nth-child(1) {
    left: 16%;
    top: 38%;
    animation-delay: -1s;
  }
  .scene-pixels i:nth-child(2) {
    left: 81%;
    top: 31%;
    animation-delay: -3.2s;
  }
  .scene-pixels i:nth-child(3) {
    left: 22%;
    top: 69%;
    animation-delay: -4s;
  }
  .scene-pixels i:nth-child(4) {
    left: 73%;
    top: 62%;
    animation-delay: -2.1s;
  }
  .scene-pixels i:nth-child(5) {
    left: 10%;
    top: 57%;
    animation-delay: -0.2s;
  }
  .scene-pixels i:nth-child(6) {
    left: 88%;
    top: 74%;
    animation-delay: -3.7s;
  }

  .name-tag {
    animation: nametag-breathe 4.8s ease-in-out infinite;
  }

  @keyframes nametag-breathe {
    0%,
    100% {
      transform: translateX(-50%) translateY(0);
    }
    50% {
      transform: translateX(-50%) translateY(-2px);
    }
  }

  @keyframes pixel-drift {
    0%,
    100% {
      transform: translateY(3px);
      opacity: 0.2;
    }
    50% {
      transform: translateY(-5px);
      opacity: 0.85;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .name-tag {
      animation: none;
    }
    .scene-pixels i {
      animation: none;
    }
  }
</style>
