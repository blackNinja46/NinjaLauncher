<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    beginMicrosoftLogin,
    completeMicrosoftLogin,
    logout,
    messageFromError,
  } from "$lib/api/tauri";
  import type { Account, LauncherSettings } from "$lib/types";
  let {
    account = $bindable(),
    settings,
    onclose,
  }: { account: Account | null; settings: LauncherSettings; onclose: () => void } = $props();
  let busy = $state(false);
  let code = $state("");
  let message = $state("");
  async function login() {
    if (!settings.microsoftClientId) {
      message = "Set your Microsoft OAuth client ID in Settings first.";
      return;
    }
    busy = true;
    message = "Requesting a secure sign-in code…";
    try {
      const device = await beginMicrosoftLogin(settings.microsoftClientId);
      code = device.user_code;
      message = device.message;
      await openUrl(device.verification_uri);
      account = await completeMicrosoftLogin(settings.microsoftClientId, device);
      onclose();
    } catch (error) {
      message = messageFromError(error);
    } finally {
      busy = false;
    }
  }
  async function signOut() {
    await logout();
    account = null;
    onclose();
  }
</script>

<div
  class="fixed inset-0 z-50 grid place-items-center bg-black/60 p-6"
  role="presentation"
  onclick={(e) => e.currentTarget === e.target && onclose()}
>
  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="account-title"
    class="mc-panel w-full max-w-md p-6 shadow-2xl shadow-blue-950/50"
  >
    <div class="flex items-start justify-between">
      <div>
        <h2 id="account-title" class="text-lg font-semibold">Minecraft account</h2>
        <p class="mt-1 text-xs text-[#777c87]">Sign in securely through Microsoft.</p>
      </div>
      <button onclick={onclose} class="rounded-lg px-2 py-1 text-[#777c87] hover:bg-white/5"
        >×</button
      >
    </div>
    {#if account}
      <div class="my-6 flex items-center gap-3 border border-cyan-300/10 bg-cyan-300/[.035] p-4">
        <div class="grid size-10 place-items-center bg-accent-500 font-bold">
          {account.username[0]}
        </div>
        <div>
          <div class="text-sm font-medium">{account.username}</div>
          <div class="text-[10px] text-[#6e8dab]">UUID {account.uuid}</div>
        </div>
      </div>
      <button
        onclick={signOut}
        class="w-full border border-red-400/20 py-3 text-xs font-semibold text-red-300 hover:bg-red-400/10"
        >Sign out</button
      >
    {:else}
      <p class="my-5 text-xs leading-5 text-[#8c919b]">
        Ninja Launcher never asks for your password. Microsoft opens in your browser and returns a
        temporary device authorization.
      </p>
      {#if code}<div class="mb-4 border border-accent-500/30 bg-accent-500/10 p-4 text-center">
          <div class="text-[10px] uppercase tracking-widest text-[#8dabc3]">Enter this code</div>
          <div class="mt-2 text-2xl font-bold tracking-[.18em]">{code}</div>
        </div>{/if}
      {#if message}<p class="mb-4 text-xs leading-5 text-[#9b9faa]">{message}</p>{/if}
      <button
        onclick={login}
        disabled={busy}
        class="mc-primary w-full py-3 text-sm font-bold disabled:opacity-60"
        >{busy ? "Waiting for Microsoft…" : "Sign in with Microsoft"}</button
      >
    {/if}
  </div>
</div>
