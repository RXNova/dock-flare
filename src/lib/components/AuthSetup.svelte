<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import type { ZoneInfo } from '$lib/types';

  let project   = $derived(store.selectedProject!);
  let authMode  = $state<'token' | 'browser'>('token');
  let apiToken  = $state('');
  let accountId = $state('');

  let working      = $state(false);   // local busy (discovery), distinct from login
  let cancelled    = $state(false);
  let cfAuthorized = $state(false);   // cert exists on disk (browser)
  let discovered   = $state<ZoneInfo | null>(null);  // zone found after browser auth
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
      const updated = { ...candidate, domain: zi.zone };
      await api.upsertProject(updated);
      store.upsertProject(updated);   // ProjectView switches to TunnelList
    } catch (e) {
      errorMsg = String(e);
    } finally {
      working = false;
    }
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
      domain: discovered.zone, browser_authed: true,
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
              <p class="text-xs font-medium text-base-content/50">Account ID</p>
              <input type="text" placeholder="a1b2c3d4…"
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

          <div class="flex justify-end">
            <button class="btn btn-primary btn-sm" onclick={connectToken}
                    disabled={!apiToken || apiToken.length < 20 || !accountId || working}>
              {#if working}<span class="loading loading-spinner loading-xs"></span>{/if}
              Connect
            </button>
          </div>

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
            <div class="rounded-lg bg-emerald-400/8 border border-emerald-400/20 px-3 py-3 space-y-1">
              <div class="flex items-center gap-2">
                <svg class="w-3.5 h-3.5 text-emerald-400 flex-shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
                  <path fill-rule="evenodd" clip-rule="evenodd"
                    d="M2.25 12c0-5.385 4.365-9.75 9.75-9.75s9.75 4.365 9.75 9.75-4.365 9.75-9.75
                       9.75S2.25 17.385 2.25 12zm13.36-1.814a.75.75 0 10-1.22-.872l-3.236 4.53L9.53
                       12.22a.75.75 0 00-1.06 1.06l2.25 2.25a.75.75 0 001.14-.094l3.75-5.25z"/>
                </svg>
                <span class="text-xs text-base-content/70">Authorized for zone</span>
              </div>
              <p class="text-sm font-mono font-semibold text-emerald-400 pl-5">{discovered.zone}</p>
              {#if discovered.all.length > 1}
                <p class="text-[10px] text-base-content/35 pl-5">
                  +{discovered.all.length - 1} more zone{discovered.all.length > 2 ? 's' : ''} on this account
                </p>
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
        <svg class="w-3 h-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path fill-rule="evenodd" clip-rule="evenodd"
            d="M16.5 4.478v.227a48.816 48.816 0 013.878.512.75.75 0 11-.256 1.478l-.209-.035-1.005
               13.07a3 3 0 01-2.991 2.77H8.084a3 3 0 01-2.991-2.77L4.087 6.66l-.209.035a.75.75
               0 01-.256-1.478A48.567 48.567 0 017.5 4.705v-.227c0-1.564 1.213-2.9 2.816-2.951a52.662
               52.662 0 013.369 0c1.603.051 2.815 1.387 2.815 2.951zm-6.136-1.452a51.196 51.196 0
               013.273 0C14.39 3.05 15 3.684 15 4.478v.113a49.488 49.488 0 00-6 0v-.113c0-.794.609-1.428
               1.364-1.452zm-.355 5.945a.75.75 0 10-1.5.058l.347 9a.75.75 0 101.499-.058l-.346-9zm5.48.058a.75.75
               0 10-1.498-.058l-.347 9a.75.75 0 001.5.058l.345-9z"/>
        </svg>
        Delete project
      </button>
    </div>

  </div>
</div>
