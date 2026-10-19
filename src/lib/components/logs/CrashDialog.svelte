<script lang="ts">
  import { openLogsFolder } from "$lib/api/tauri";
  import Icon from "$lib/components/ui/Icon.svelte";
  import type { GameCrashDetails } from "$lib/types";

  let {
    crash,
    onclose,
    onopenconsole,
    onrepair,
    onupload,
  }: {
    crash: GameCrashDetails;
    onclose: () => void;
    onopenconsole: (instanceId: string) => void;
    onrepair?: (instanceId: string) => Promise<void> | void;
    onupload?: (crash: GameCrashDetails) => Promise<void>;
  } = $props();

  let copyState = $state<"idle" | "copied" | "failed">("idle");
  let repairing = $state(false);
  let repairError = $state("");
  let uploadState = $state<"idle" | "uploading" | "uploaded" | "failed">("idle");
  let uploadError = $state("");

  async function repairProfile() {
    if (!onrepair || repairing) return;
    repairing = true;
    repairError = "";
    try {
      await onrepair(crash.instanceId);
    } catch (cause) {
      repairError =
        typeof cause === "object" && cause && "message" in cause
          ? String(cause.message)
          : typeof cause === "string"
            ? cause
            : "The profile could not be repaired safely.";
    } finally {
      repairing = false;
    }
  }

  async function uploadReport() {
    if (!onupload || uploadState === "uploading" || uploadState === "uploaded") return;
    uploadState = "uploading";
    uploadError = "";
    try {
      await onupload(crash);
      uploadState = "uploaded";
    } catch (cause) {
      uploadState = "failed";
      uploadError =
        typeof cause === "object" && cause && "message" in cause
          ? String(cause.message)
          : typeof cause === "string"
            ? cause
            : "The crash report could not be uploaded.";
    }
  }

  function crashReport(): string {
    const lines = [
      "Ninja Launcher · Minecraft error report",
      "========================================",
      `Created: ${new Date().toISOString()}`,
      `Error: ${crash.title}`,
      `Profile: ${crash.profileName}`,
      `Profile ID: ${crash.instanceId}`,
      `Minecraft: ${crash.minecraftVersion}`,
      `Exit code: ${crash.exitCode ?? "unknown / process did not start"}`,
      "",
      "Summary",
      crash.message,
      "",
      "Recommended next step",
      crash.suggestion,
    ];
    if (crash.technicalDetail) {
      lines.push("", "Detected technical error", crash.technicalDetail);
    }
    if (crash.logExcerpt.length) {
      lines.push("", "Last console messages", "---------------------", ...crash.logExcerpt);
    }
    return lines.join("\n");
  }

  async function copyErrorDetails() {
    const report = crashReport();
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(report);
      } else {
        const field = document.createElement("textarea");
        field.value = report;
        field.style.position = "fixed";
        field.style.opacity = "0";
        document.body.appendChild(field);
        field.select();
        const copied = document.execCommand("copy");
        field.remove();
        if (!copied) throw new Error("Clipboard copy was rejected.");
      }
      copyState = "copied";
      window.setTimeout(() => (copyState = "idle"), 2_000);
    } catch {
      copyState = "failed";
      window.setTimeout(() => (copyState = "idle"), 2_500);
    }
  }
</script>

<svelte:window onkeydown={(event) => event.key === "Escape" && onclose()} />

<div
  class="crash-backdrop fixed inset-0 z-[110] grid place-items-center overflow-hidden rounded-[12px] bg-black/70 p-6 backdrop-blur-xl [.window-maximized_&]:rounded-none"
  role="presentation"
  onclick={(event) => event.currentTarget === event.target && onclose()}
>
  <div
    class="crash-dialog w-full max-w-[620px] overflow-hidden rounded-[14px] border border-white/[.11] bg-[var(--color-section-background)] shadow-[0_30px_100px_rgba(0,0,0,.72)]"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="crash-dialog-title"
  >
    <div
      class="relative h-44 overflow-hidden border-b border-white/[.08] bg-[var(--color-navy-900)]"
    >
      <img
        src="/backgrounds/minecraft-bedrock-wallpaper.png"
        alt=""
        class="absolute inset-0 size-full object-cover object-[center_47%] contrast-[1.03] saturate-[.88]"
      />
      <div class="crash-art-shade absolute inset-0"></div>
      <div
        class="absolute inset-0 bg-gradient-to-t from-[var(--color-section-background)] via-transparent to-black/10"
      ></div>
      <div class="absolute inset-0 bg-red-950/15 mix-blend-color"></div>
      <div class="absolute inset-x-0 bottom-0 flex items-end gap-4 px-7 pb-6">
        <span
          class="grid size-12 shrink-0 place-items-center rounded-[9px] border border-red-300/25 bg-red-400/15 text-red-200 shadow-[0_10px_30px_rgba(239,68,68,.16)]"
        >
          <Icon name="warning" size={21} />
        </span>
        <div class="min-w-0">
          <p class="text-[9px] font-bold uppercase tracking-[.18em] text-red-200/65">
            Minecraft stopped unexpectedly
          </p>
          <h2 id="crash-dialog-title" class="mt-1 truncate text-2xl font-black text-white">
            {crash.title}
          </h2>
        </div>
      </div>
      <button
        type="button"
        onclick={onclose}
        class="absolute right-4 top-4 grid size-9 place-items-center rounded-[7px] border border-white/10 bg-black/35 text-white/65 backdrop-blur-md transition hover:bg-white/10 hover:text-white"
        aria-label="Close crash dialog"
        title="Close"
      >
        <Icon name="close" size={15} />
      </button>
    </div>

    <div class="p-7">
      <p class="text-sm leading-6 text-[var(--color-text-secondary)]">{crash.message}</p>

      <div class="mt-5 grid grid-cols-2 gap-3">
        <div class="rounded-[8px] border border-white/[.08] bg-white/[.025] px-4 py-3">
          <div
            class="text-[8px] font-bold uppercase tracking-[.14em] text-[var(--color-text-faint)]"
          >
            Profile
          </div>
          <div class="mt-1 truncate text-xs font-bold text-white">{crash.profileName}</div>
        </div>
        <div class="rounded-[8px] border border-white/[.08] bg-white/[.025] px-4 py-3">
          <div
            class="text-[8px] font-bold uppercase tracking-[.14em] text-[var(--color-text-faint)]"
          >
            Version
          </div>
          <div class="mt-1 flex items-center justify-between gap-2 text-xs font-bold text-white">
            <span>Minecraft {crash.minecraftVersion}</span>
            <span class="text-[10px] font-medium text-red-200/65"
              >Exit {crash.exitCode ?? "unknown"}</span
            >
          </div>
        </div>
      </div>

      <div
        class="mt-3 rounded-[8px] border border-[var(--color-primary)]/15 bg-[var(--color-primary)]/[.055] px-4 py-3"
      >
        <div class="text-[8px] font-bold uppercase tracking-[.14em] text-[var(--color-primary)]">
          Recommended next step
        </div>
        <p class="mt-1.5 text-xs leading-5 text-[var(--color-text-secondary)]">
          {crash.suggestion}
        </p>
      </div>

      {#if crash.technicalDetail}
        <div
          class="mt-3 max-h-24 overflow-auto break-words rounded-[8px] border border-white/[.07] bg-black/20 px-4 py-3 font-mono text-[10px] leading-5 text-[var(--color-text-muted)]"
        >
          {crash.technicalDetail}
        </div>
      {/if}

      {#if repairError}
        <p class="mt-3 text-xs leading-5 text-[var(--color-danger)]" role="alert">
          <span class="font-bold">Automatic repair stopped:</span>
          {repairError}
        </p>
      {/if}

      {#if uploadError}
        <p class="mt-3 text-xs leading-5 text-[var(--color-danger)]" role="alert">
          {uploadError}
        </p>
      {/if}

      <div class="mt-6 border-t border-white/[.07] pt-5">
        <div class="grid grid-cols-2 gap-2 {onupload ? 'sm:grid-cols-3' : 'sm:grid-cols-2'}">
          <button
            type="button"
            onclick={copyErrorDetails}
            class="mc-secondary flex h-10 items-center justify-center gap-2 px-3 text-[11px] font-bold {copyState ===
            'copied'
              ? 'text-[var(--color-success)]'
              : copyState === 'failed'
                ? 'text-[var(--color-danger)]'
                : ''}"
            aria-live="polite"
          >
            <Icon name={copyState === "copied" ? "check" : "copy"} size={12} />
            {copyState === "copied"
              ? "Copied"
              : copyState === "failed"
                ? "Copy failed"
                : "Copy details"}
          </button>
          {#if onupload}
            <button
              type="button"
              onclick={uploadReport}
              disabled={uploadState === "uploading" || uploadState === "uploaded"}
              class="mc-secondary flex h-10 items-center justify-center gap-2 px-3 text-[11px] font-bold disabled:cursor-wait disabled:opacity-60 {uploadState ===
              'uploaded'
                ? 'text-[var(--color-success)]'
                : uploadState === 'failed'
                  ? 'text-[var(--color-danger)]'
                  : ''}"
            >
              <Icon name={uploadState === "uploaded" ? "check" : "upload"} size={12} />
              {uploadState === "uploading"
                ? "Uploading…"
                : uploadState === "uploaded"
                  ? "Uploaded"
                  : uploadState === "failed"
                    ? "Retry upload"
                    : "Upload report"}
            </button>
          {/if}
          <button
            type="button"
            onclick={openLogsFolder}
            class="mc-secondary col-span-2 flex h-10 items-center justify-center gap-2 px-3 text-[11px] font-bold sm:col-span-1"
          >
            <Icon name="folder" size={12} /> Open log folder
          </button>
        </div>

        <div class="mt-3 flex items-center justify-end gap-2">
          <button type="button" onclick={onclose} class="mc-secondary h-11 px-5 text-xs font-bold">
            Close
          </button>
          {#if crash.repairAvailable && onrepair}
            <button
              type="button"
              onclick={() => onopenconsole(crash.instanceId)}
              class="mc-secondary flex h-11 items-center gap-2 px-5 text-xs font-bold"
            >
              <Icon name="logs" size={13} /> View console
            </button>
            <button
              type="button"
              onclick={repairProfile}
              disabled={repairing}
              class="mc-primary flex h-11 min-w-36 items-center justify-center gap-2 px-5 text-xs font-bold disabled:cursor-wait disabled:opacity-60"
            >
              <Icon name="settings" size={13} />
              {repairing ? "Repairing…" : "Repair profile"}
            </button>
          {:else}
            <button
              type="button"
              onclick={() => onopenconsole(crash.instanceId)}
              class="mc-primary flex h-11 items-center gap-2 px-5 text-xs font-bold"
            >
              <Icon name="logs" size={13} /> Open console
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .crash-backdrop {
    animation: crash-backdrop-in 180ms ease-out both;
  }

  .crash-dialog {
    animation: crash-dialog-in 320ms cubic-bezier(0.16, 1, 0.3, 1) both;
  }

  .crash-art-shade {
    background: linear-gradient(
      90deg,
      color-mix(in srgb, var(--color-navy-900) 76%, transparent) 0%,
      color-mix(in srgb, var(--color-navy-900) 44%, transparent) 48%,
      color-mix(in srgb, var(--color-navy-900) 16%, transparent) 100%
    );
  }

  @keyframes crash-backdrop-in {
    from {
      opacity: 0;
    }
  }

  @keyframes crash-dialog-in {
    from {
      opacity: 0;
      transform: translateY(18px) scale(0.97);
    }
  }

  :global(.animations-disabled) .crash-backdrop,
  :global(.animations-disabled) .crash-dialog {
    animation: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .crash-backdrop,
    .crash-dialog {
      animation: none;
    }
  }
</style>
