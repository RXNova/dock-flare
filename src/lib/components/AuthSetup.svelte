<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import type { ZoneInfo } from '$lib/types';
  import { CircleCheck, Trash2 } from '@lucide/svelte';

  let project   = $derived(store.selectedProject!);
  let authMode  = $state<'token' | 'browser'>('token');
  let apiToken  = $state('');
  let accountId = $state('');

  let working      = $state(false);   // local busy (discovery), distinct from login
  let cancelled    = $state(false);
  let cfAuthorized = $state(false);   // cert exists on disk (browser)
  let discovered   = $state<ZoneInfo | null>(null);  // zone found after browser auth
  let selectedZone = $state('');
  let errorMsg     = $state<string | null>(null);

  // Sync local fields when the selected project changes
  $effect(() => {
    if (!project) return;
    authMode     = project.auth_mode as 'token' | 'browser';
    apiToken     = project.api_token  ?? '';
    accountId    = project.account_id ?? '';
    discovered   = null;
    errorMsg     = null;
    cfAuthorized = false;
    if (project.auth_mode === 'browser') {
      api.checkCfAuth(project.id).then(v => { cfAuthorized = v; });
    }
  });

  // ── Token mode: connect = discover the zone, then persist ──────────────────
  async function connectToken() {
    working = true;
    errorMsg = null;
    const candidate = {
      ...project, auth_mode: 'token' as const,
      api_token: apiToken, account_id: accountId,
    };
    try {
      const zi = await api.discoverZone(candidate);
      // Use auto-discovered account_id if the user left it blank
      const resolvedAccountId = zi.account_id || accountId;
      if (zi.all.length > 1) {
        // Multiple zones — show picker before saving
        discovered = zi;
        selectedZone = zi.zone;
        accountId = resolvedAccountId;
      } else {
        const updated = { ...candidate, domain: zi.zone, account_id: resolvedAccountId };
        await api.upsertProject(updated);
        store.upsertProject(updated);   // ProjectView switches to TunnelList
      }
    } catch (e) {
      errorMsg = String(e);
    } finally {
      working = false;
    }
  }

  async function commitToken() {
    const updated = {
      ...project, auth_mode: 'token' as const,
      api_token: apiToken, account_id: accountId,
      domain: selectedZone,
    };
    await api.upsertProject(updated);
    store.upsertProject(updated);
  }

  // ── Browser mode ───────────────────────────────────────────────────────────
  async function authorize() {
    cancelled = false;
    errorMsg = null;
    discovered = null;
    store.status = 'processing';
    store.clearLogs();
    try {
      await api.cloudflaredLogin(project.id);
      cfAuthorized = true;
      store.status = 'idle';
      // Discover which zone the cert is for — this becomes the project's domain
      working = true;
      try {
        discovered = await api.discoverZone({ ...project, auth_mode: 'browser' });
        selectedZone = discovered.zone;
      } catch (e) {
        errorMsg = String(e);
      } finally {
        working = false;
      }
    } catch (e) {
      if (cancelled) {
        store.status = 'idle';
      } else {
        store.status = 'failed';
        store.appendLog(`FATAL: ${e}`);
      }
    }
  }

  async function commitBrowser() {
    if (!discovered) return;
    const updated = {
      ...project, auth_mode: 'browser' as const,
      domain: selectedZone || discovered.zone, browser_authed: true,
    };
    await api.upsertProject(updated);
    store.upsertProject(updated);   // ProjectView switches to TunnelList
  }

  async function cancelLogin() {
    cancelled = true;
    await api.cancelLogin();
    store.status = 'idle';
    store.clearLogs();
  }

  async function retryDiscover() {
    working = true;
    errorMsg = null;
    try {
      discovered = await api.discoverZone({ ...project, auth_mode: 'browser' });
      selectedZone = discovered.zone;
    } catch (e) {
      errorMsg = String(e);
    } finally {
      working = false;
    }
  }

  async function installCloudflared() {
    store.status = 'processing';
    store.clearLogs();
    try {
      await api.installCloudflared();
      store.cloudflaredFound = true;
      store.status = 'idle';
    } catch (e) {
      store.status = 'failed';
      store.appendLog(`FATAL: ${e}`);
    }
  }

  async function deleteProject() {
    await api.deleteProject(project.id);
    store.removeProject(project.id);
  }

  function cancel() { store.selectedId = null; }
</script>

<div class="flex-1 overflow-y-auto p-5">
  <div class="max-w-lg mx-auto space-y-4">

    <!-- Heading -->
    <div class="flex items-start justify-between">
      <div>
        <h2 class="text-sm font-semibold">{project.name || 'Untitled project'}</h2>
        <p class="text-xs text-base-content/40 mt-0.5">
          Authenticate to detect this project's domain and manage tunnels.
        </p>
      </div>
      <button
        class="btn btn-ghost btn-xs text-base-content/30 hover:text-base-content/60 mt-0.5"
        onclick={store.busy ? cancelLogin : cancel}
      >✕ {store.busy ? 'Stop' : 'Cancel'}</button>
    </div>

    <!-- Auth card -->
    <div class="rounded-xl border border-base-300 bg-base-200 overflow-hidden">

      <!-- Mode switcher -->
      <div class="px-4 pt-3.5 pb-3 border-b border-base-300 flex items-center justify-between">
        <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
          Authentication method
        </span>
        <div class="flex bg-base-300/60 rounded-lg p-0.5 gap-px">
          {#each [['token','API Token'],['browser','Browser']] as [m, label] (m)}
            <button
              class="px-3 py-1 rounded-[6px] text-xs font-medium transition-all duration-150
                     {authMode === m
                       ? 'bg-base-100 shadow-sm text-base-content'
                       : 'text-base-content/40 hover:text-base-content/60'}"
              onclick={() => { authMode = m as 'token' | 'browser'; discovered = null; errorMsg = null; }}
              disabled={store.busy || working}
            >{label}</button>
          {/each}
        </div>
      </div>

      <div class="p-4 space-y-4">

        <!-- ── API Token ── -->
        {#if authMode === 'token'}
          <div class="grid grid-cols-2 gap-3">
            <div class="space-y-1.5">
              <p class="text-xs font-medium text-base-content/50">API Token</p>
              <input type="password" placeholder="cf_api_token_…"
                bind:value={apiToken} disabled={working}
                class="input input-sm w-full bg-base-100 border-base-300
                       focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50
                       font-mono text-[11px]" />
            </div>
            <div class="space-y-1.5">
              <p class="text-xs font-medium text-base-content/50">Account ID <span class="text-base-content/30 font-normal">(optional)</span></p>
              <input type="text" placeholder="auto-detected from token"
                bind:value={accountId} disabled={working}
                class="input input-sm w-full bg-base-100 border-base-300
                       focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50
                       font-mono text-[11px]" />
            </div>
          </div>
          <p class="text-[11px] text-base-content/35">
            Token needs <span class="font-mono text-base-content/50">Tunnel:Edit</span>,
            <span class="font-mono text-base-content/50">DNS:Edit</span> and
            <span class="font-mono text-base-content/50">Zone:Read</span> permissions.
          </p>

          {#if errorMsg}
            <div class="rounded-lg bg-red-400/8 border border-red-400/25 px-3 py-2">
              <p class="text-[11px] text-red-400">{errorMsg}</p>
            </div>
          {/if}

          {#if apiToken && apiToken.length < 20}
            <p class="text-[11px] text-amber-400">Token looks too short — Cloudflare tokens are usually 40+ characters</p>
          {/if}

          {#if discovered && discovered.all.length > 1}
            <!-- Zone picker — shown after discovery when token covers multiple zones -->
            <div class="rounded-lg bg-emerald-400/8 border border-emerald-400/20 px-3 py-3 space-y-2">
              <p class="text-xs text-base-content/70">Multiple zones found — choose one:</p>
              <select bind:value={selectedZone}
                class="select select-sm w-full bg-base-100 border-base-300 font-mono
                       focus:outline-none focus:ring-2 focus:ring-primary/25">
                {#each discovered.all as z}
                  <option value={z}>{z}</option>
                {/each}
              </select>
            </div>
            <div class="flex justify-end gap-2">
              <button class="btn btn-ghost btn-sm text-base-content/50"
                      onclick={() => { discovered = null; }}
                      disabled={working}>Back</button>
              <button class="btn btn-primary btn-sm" onclick={commitToken} disabled={working}>
                Save Project
              </button>
            </div>
          {:else}
            <div class="flex justify-end">
              <button class="btn btn-primary btn-sm" onclick={connectToken}
                      disabled={!apiToken || apiToken.length < 20 || working}>
                {#if working}<span class="loading loading-spinner loading-xs"></span>{/if}
                Connect
              </button>
            </div>
          {/if}

        <!-- ── Browser ── -->
        {:else}
          {#if store.cloudflaredFound === false}
            <div class="flex items-center gap-3 rounded-lg bg-base-300/50 border border-base-300 px-3 py-2.5">
              <span class="w-2 h-2 rounded-full bg-red-400 flex-shrink-0"></span>
              <span class="text-xs text-base-content/50 flex-1">
                <code class="font-mono text-base-content/70">cloudflared</code> not found
              </span>
              <button class="btn btn-xs btn-primary" onclick={installCloudflared} disabled={store.busy}>
                {#if store.busy}<span class="loading loading-spinner loading-xs"></span>{/if}
                Install via Homebrew
              </button>
            </div>

          {:else if store.busy}
            <!-- Login in progress -->
            <div class="rounded-lg bg-base-300/50 border border-amber-400/20 px-3 py-3 space-y-2.5">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2.5">
                  <span class="loading loading-spinner loading-xs text-amber-400 flex-shrink-0"></span>
                  <span class="text-xs font-medium text-base-content/70">Waiting for browser authorization…</span>
                </div>
                <button class="btn btn-ghost btn-xs text-red-400/60 hover:text-red-400" onclick={cancelLogin}>
                  Cancel login
                </button>
              </div>
              <p class="text-[11px] text-base-content/40 pl-5">
                If the browser didn't open, click the URL in the terminal log below.
              </p>
            </div>

          {:else if working}
            <!-- Discovering zone -->
            <div class="flex items-center gap-2 text-[11px] text-base-content/40 px-1">
              <span class="loading loading-spinner loading-xs"></span>
              Detecting authorized domain…
            </div>

          {:else if discovered}
            <!-- Zone discovered — confirm -->
            <div class="rounded-lg bg-emerald-400/8 border border-emerald-400/20 px-3 py-3 space-y-2">
              <div class="flex items-center gap-2">
                <CircleCheck class="w-3.5 h-3.5 text-emerald-400 flex-shrink-0" strokeWidth={2} />
                <span class="text-xs text-base-content/70">Authorized for zone</span>
              </div>
              {#if discovered.all.length > 1}
                <select bind:value={selectedZone}
                  class="select select-sm w-full bg-base-100 border-base-300 font-mono text-sm
                         focus:outline-none focus:ring-2 focus:ring-primary/25 ml-5">
                  {#each discovered.all as z}
                    <option value={z}>{z}</option>
                  {/each}
                </select>
              {:else}
                <p class="text-sm font-mono font-semibold text-emerald-400 pl-5">{discovered.zone}</p>
              {/if}
            </div>
            <div class="flex justify-end gap-2">
              <button class="btn btn-ghost btn-sm text-base-content/50" onclick={authorize}>
                Not right? Re-authorize
              </button>
              <button class="btn btn-primary btn-sm" onclick={commitBrowser}>Continue</button>
            </div>

          {:else}
            <!-- Not yet authorized / cert present -->
            <div class="flex items-center gap-3 rounded-lg bg-base-300/50 border border-base-300 px-3 py-2.5">
              <span class="w-2 h-2 rounded-full {cfAuthorized ? 'bg-emerald-400' : 'bg-amber-400'} flex-shrink-0"></span>
              <span class="text-xs text-base-content/50 flex-1">
                {cfAuthorized ? 'Cert present — detect domain' : 'Not yet authorized'}
              </span>
              {#if cfAuthorized}
                <button class="btn btn-xs btn-primary" onclick={retryDiscover}>
                  Detect domain
                </button>
              {:else}
                <button class="btn btn-xs btn-primary" onclick={authorize}>
                  Authorize with Cloudflare
                </button>
              {/if}
            </div>
            {#if errorMsg}
              <div class="rounded-lg bg-red-400/8 border border-red-400/25 px-3 py-2 space-y-2">
                <p class="text-[11px] text-red-400">{errorMsg}</p>
                {#if cfAuthorized}
                  <button class="text-[10px] text-base-content/40 hover:text-base-content/60 underline"
                          onclick={authorize}>
                    Re-authorize with Cloudflare instead
                  </button>
                {/if}
              </div>
            {/if}
          {/if}

          <p class="text-[11px] text-base-content/35">
            Opens a browser window to log in to Cloudflare. The domain is detected from your login.
          </p>
        {/if}

      </div>
    </div>

    <!-- Delete project -->
    <div class="flex items-center justify-between pt-1">
      <p class="text-[11px] text-base-content/25">Don't need this project?</p>
      <button
        class="btn btn-ghost btn-xs text-red-400/50 hover:text-red-400 hover:bg-red-400/10"
        onclick={deleteProject}
        disabled={store.busy || working}
      >
        <Trash2 class="w-3 h-3" strokeWidth={2} />
        Delete project
      </button>
    </div>

  </div>
</div>
