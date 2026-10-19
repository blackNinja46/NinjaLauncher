<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    beginMicrosoftLogin,
    completeMicrosoftLogin,
    getAccounts,
    getNetworkRanks,
    messageFromError,
    removeMicrosoftAccount,
    switchMicrosoftAccount,
  } from "$lib/api/tauri";
  import {
    hasNinjaPlus,
    isNameGradientUnlocked,
    nameGradient,
    rankLogoColor,
    type NameGradient,
  } from "$lib/nameGradients";
  import type { Account, PlayerRanks } from "$lib/types";
  import MinecraftAvatar from "$lib/components/account/MinecraftAvatar.svelte";
  let {
    account = $bindable(),
    accounts = $bindable(),
    networkRanks,
    onclose,
  }: {
    account: Account | null;
    accounts: Account[];
    networkRanks: PlayerRanks | null;
    onclose: () => void;
  } = $props();
  let busy = $state(false);
  let adding = $state(false);
  let code = $state("");
  let message = $state("");
  let ranksByAccount = $state<Record<string, PlayerRanks | null>>({});
  const requestedRankIds = new Set<string>();

  function ranksFor(savedAccount: Account): PlayerRanks | null {
    if (networkRanks?.uuid.replaceAll("-", "") === savedAccount.uuid.replaceAll("-", "")) {
      return networkRanks;
    }
    return ranksByAccount[savedAccount.uuid] ?? null;
  }

  function gradientFor(savedAccount: Account, ranks: PlayerRanks | null): NameGradient {
    const stored =
      localStorage.getItem(`ninja-launcher:name-gradient:${savedAccount.uuid}`) ?? "none";
    const gradient = nameGradient(stored);
    return isNameGradientUnlocked(gradient, ranks) ? gradient : nameGradient("none");
  }

  async function loadAccountRanks(savedAccount: Account) {
    requestedRankIds.add(savedAccount.uuid);
    try {
      const ranks = await getNetworkRanks(savedAccount.uuid);
      ranksByAccount = { ...ranksByAccount, [savedAccount.uuid]: ranks };
    } catch {
      ranksByAccount = { ...ranksByAccount, [savedAccount.uuid]: null };
    }
  }

  $effect(() => {
    if (networkRanks) {
      const matching = accounts.find(
        (saved) => saved.uuid.replaceAll("-", "") === networkRanks.uuid.replaceAll("-", ""),
      );
      if (matching && ranksByAccount[matching.uuid] !== networkRanks) {
        ranksByAccount = { ...ranksByAccount, [matching.uuid]: networkRanks };
      }
    }
    for (const savedAccount of accounts) {
      if (!(savedAccount.uuid in ranksByAccount) && !requestedRankIds.has(savedAccount.uuid)) {
        void loadAccountRanks(savedAccount);
      }
    }
  });

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
  class="fixed inset-0 z-50 grid place-items-center bg-black/70 p-6 backdrop-blur-sm"
  role="presentation"
  onclick={(e) => e.currentTarget === e.target && onclose()}
>
  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="account-title"
    class="account-dialog max-h-[85vh] w-full max-w-[540px] overflow-y-auto rounded-xl border border-white/[.12] bg-[#101218]/[.98] shadow-[0_28px_90px_rgba(0,0,0,.58),inset_0_1px_0_rgba(255,255,255,.07)]"
  >
    <div class="flex items-start justify-between border-b border-white/[.08] px-6 py-5">
      <div>
        <p class="text-[9px] font-bold uppercase tracking-[.16em] text-[var(--color-text-faint)]">
          Account
        </p>
        <h2 id="account-title" class="mt-1 text-lg font-bold tracking-[-.025em]">
          Minecraft accounts
        </h2>
        <p class="mt-1.5 text-xs text-[var(--color-text-muted)]">
          Add accounts and switch with one click.
        </p>
      </div>
      <button
        onclick={onclose}
        aria-label="Close account menu"
        class="grid size-8 place-items-center rounded-md border border-transparent text-lg text-[var(--color-text-faint)] hover:border-white/[.08] hover:bg-white/[.06] hover:text-white"
        >×</button
      >
    </div>
    <div class="p-6">
      {#if accounts.length > 0}
        <div class="mb-5 space-y-2">
          {#each accounts as savedAccount (savedAccount.uuid)}
            {@const savedRanks = ranksFor(savedAccount)}
            {@const savedGradient = gradientFor(savedAccount, savedRanks)}
            <div
              class="flex items-center gap-3 rounded-lg border p-3 transition-colors {account?.uuid ===
              savedAccount.uuid
                ? 'border-white/[.16] bg-white/[.085] shadow-[inset_0_1px_0_rgba(255,255,255,.06)]'
                : 'border-white/[.08] bg-white/[.025] hover:border-white/[.13] hover:bg-white/[.045]'}"
            >
              <div class="size-10 shrink-0 overflow-hidden rounded-md">
                <MinecraftAvatar account={savedAccount} />
              </div>
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <span class="relative grid size-7 shrink-0 place-items-center" aria-hidden="true">
                    <span
                      class="account-rank-logo absolute inset-0"
                      style={`--account-rank-color: ${rankLogoColor(savedRanks)}`}
                    ></span>
                    {#if hasNinjaPlus(savedRanks)}
                      <span
                        class="absolute right-[1px] bottom-[2px] z-10 text-[14px] font-black leading-none text-white drop-shadow-[0_1px_2px_rgba(0,0,0,.95)]"
                        >+</span
                      >
                    {/if}
                  </span>
                  <div
                    class:account-name-gradient={savedGradient.id !== "none"}
                    class="truncate text-sm font-bold"
                    style={savedGradient.id === "none"
                      ? undefined
                      : `--account-gradient-colors: ${savedGradient.colors.join(", ")}; --account-gradient-glow: ${savedGradient.colors[1]}`}
                  >
                    {savedAccount.username}
                  </div>
                  {#if account?.uuid === savedAccount.uuid}
                    <span
                      class="rounded-full border border-white/[.12] bg-white/[.08] px-2 py-0.5 text-[8px] font-bold uppercase tracking-wider text-[var(--color-text-secondary)]"
                      >Active</span
                    >
                  {/if}
                </div>
                <div class="mt-0.5 truncate text-[9px] text-[var(--color-text-faint)]">
                  {savedAccount.uuid}
                </div>
              </div>
              {#if account?.uuid !== savedAccount.uuid}
                <button
                  onclick={() => selectAccount(savedAccount.uuid)}
                  disabled={busy}
                  class="mc-secondary shrink-0 px-3 py-2 text-[10px] font-bold disabled:opacity-50"
                  >Switch</button
                >
              {/if}
              <button
                onclick={() => removeAccount(savedAccount.uuid)}
                disabled={busy}
                aria-label={`Remove ${savedAccount.username}`}
                title="Remove account"
                class="mc-remove shrink-0 px-2.5 py-2 text-xs disabled:opacity-50">×</button
              >
            </div>
          {/each}
        </div>
      {/if}

      {#if accounts.length === 0 || adding}
        <p class="mb-5 text-xs leading-5 text-[var(--color-text-muted)]">
          Ninja Launcher never asks for your password. Microsoft opens in your browser and returns a
          temporary device authorization.
        </p>
        {#if code}<div
            class="mb-4 rounded-lg border border-white/[.12] bg-white/[.05] p-4 text-center"
          >
            <div
              class="text-[10px] font-bold uppercase tracking-widest text-[var(--color-text-muted)]"
            >
              Enter this code
            </div>
            <div class="mt-2 text-2xl font-bold tracking-[.18em]">{code}</div>
          </div>{/if}
        {#if message}<p class="mb-4 text-xs leading-5 text-[var(--color-text-secondary)]">
            {message}
          </p>{/if}
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
            class="mc-secondary mt-2 w-full py-2.5 text-xs text-[var(--color-text-secondary)] disabled:opacity-50"
            >Cancel</button
          >
        {/if}
      {:else}
        {#if message}<p class="mb-4 text-xs leading-5 text-[var(--color-text-secondary)]">
            {message}
          </p>{/if}
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
</div>

<style>
  .account-rank-logo {
    background-color: var(--account-rank-color);
    filter: drop-shadow(0 2px 7px color-mix(in srgb, var(--account-rank-color) 34%, transparent));
    -webkit-mask: url("/brand/ninja-launcher-app-icon.png") center / contain no-repeat;
    mask: url("/brand/ninja-launcher-app-icon.png") center / contain no-repeat;
  }

  .account-name-gradient {
    color: transparent;
    background-image: linear-gradient(90deg, var(--account-gradient-colors));
    background-size: 160% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    filter: saturate(1.25) brightness(1.1)
      drop-shadow(0 1px 5px color-mix(in srgb, var(--account-gradient-glow) 32%, transparent));
    animation: account-name-gradient-flow 1.6s ease-in-out infinite alternate;
  }

  @keyframes account-name-gradient-flow {
    from {
      background-position: 0% 50%;
    }
    to {
      background-position: 100% 50%;
    }
  }

  :global(.animations-disabled) .account-name-gradient {
    animation: none;
    background-position: 50% 50%;
  }

  @media (prefers-reduced-motion: reduce) {
    .account-name-gradient {
      animation: none;
    }
  }
</style>
