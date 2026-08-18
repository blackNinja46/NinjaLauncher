<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    beginMicrosoftLogin,
    completeMicrosoftLogin,
    getAccounts,
    messageFromError,
    removeMicrosoftAccount,
    switchMicrosoftAccount,
  } from "$lib/api/tauri";
  import type { Account } from "$lib/types";
  import MinecraftAvatar from "$lib/components/account/MinecraftAvatar.svelte";
  let {
    account = $bindable(),
    accounts = $bindable(),
    onclose,
  }: { account: Account | null; accounts: Account[]; onclose: () => void } = $props();
  let busy = $state(false);
  let adding = $state(false);
  let code = $state("");
  let message = $state("");

  function applyAccountState(state: { accounts: Account[]; activeUuid: string | null }) {
    accounts = state.accounts;
    account = accounts.find((item) => item.uuid === state.activeUuid) ?? accounts[0] ?? null;
  }

  async function login() {
    busy = true;
    code = "";
    message = "Requesting a secure sign-in code…";
    try {
      const device = await beginMicrosoftLogin();
      code = device.user_code;
      message = device.message;
      await openUrl(device.verification_uri);
      await completeMicrosoftLogin(device);
      applyAccountState(await getAccounts());
      adding = false;
      code = "";
      message = "Account added successfully.";
    } catch (error) {
      message = messageFromError(error);
    } finally {
      busy = false;
    }
  }

  async function selectAccount(uuid: string) {
    busy = true;
    message = "";
    try {
      account = await switchMicrosoftAccount(uuid);
    } catch (error) {
      message = messageFromError(error);
    } finally {
      busy = false;
    }
  }

  async function removeAccount(uuid: string) {
    busy = true;
    message = "";
    try {
      applyAccountState(await removeMicrosoftAccount(uuid));
      if (accounts.length === 0) adding = true;
    } catch (error) {
      message = messageFromError(error);
    } finally {
      busy = false;
    }
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
    class="mc-panel max-h-[85vh] w-full max-w-lg overflow-y-auto p-6 shadow-2xl shadow-blue-950/50"
  >
    <div class="flex items-start justify-between">
      <div>
        <h2 id="account-title" class="text-lg font-semibold">Minecraft accounts</h2>
        <p class="mt-1 text-xs text-[#777c87]">Add accounts and switch with one click.</p>
      </div>
      <button onclick={onclose} class="rounded-lg px-2 py-1 text-[#777c87] hover:bg-white/5"
        >×</button
      >
    </div>
    {#if accounts.length > 0}
      <div class="my-5 space-y-2">
        {#each accounts as savedAccount (savedAccount.uuid)}
          <div
            class="flex items-center gap-3 border p-3 {account?.uuid === savedAccount.uuid
              ? 'border-cyan-300/35 bg-cyan-300/[.08]'
              : 'border-cyan-300/10 bg-cyan-300/[.025]'}"
          >
            <div class="size-10 shrink-0">
              <MinecraftAvatar account={savedAccount} />
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <div class="truncate text-sm font-medium">{savedAccount.username}</div>
                {#if account?.uuid === savedAccount.uuid}
                  <span
                    class="border border-cyan-300/20 bg-cyan-300/10 px-1.5 py-0.5 text-[8px] font-bold uppercase tracking-wider text-cyan-200"
                    >Active</span
                  >
                {/if}
              </div>
              <div class="mt-0.5 truncate text-[9px] text-[#6e8dab]">{savedAccount.uuid}</div>
            </div>
            {#if account?.uuid !== savedAccount.uuid}
              <button
                onclick={() => selectAccount(savedAccount.uuid)}
                disabled={busy}
                class="mc-primary shrink-0 px-3 py-2 text-[10px] font-bold disabled:opacity-50"
                >Switch</button
              >
            {/if}
            <button
              onclick={() => removeAccount(savedAccount.uuid)}
              disabled={busy}
              aria-label={`Remove ${savedAccount.username}`}
              title="Remove account"
              class="shrink-0 border border-red-400/20 px-2.5 py-2 text-xs text-red-300 hover:bg-red-400/10 disabled:opacity-50"
              >×</button
            >
          </div>
        {/each}
      </div>
    {/if}

    {#if accounts.length === 0 || adding}
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
        >{busy ? "Waiting for Microsoft…" : "Add Microsoft account"}</button
      >
      {#if accounts.length > 0}
        <button
          onclick={() => {
            adding = false;
            code = "";
            message = "";
          }}
          disabled={busy}
          class="mt-2 w-full border border-cyan-300/10 py-2.5 text-xs text-[#88a8c4] hover:bg-white/5 disabled:opacity-50"
          >Cancel</button
        >
      {/if}
    {:else}
      {#if message}<p class="mb-4 text-xs leading-5 text-[#9b9faa]">{message}</p>{/if}
      <button
        onclick={() => {
          adding = true;
          code = "";
          message = "";
        }}
        class="mc-primary w-full py-3 text-sm font-bold">+ Add another account</button
      >
    {/if}
  </div>
</div>
