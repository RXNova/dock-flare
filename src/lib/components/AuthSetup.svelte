<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';

  let project      = $derived(store.selectedProject!);
  let saving       = $state(false);
  let cfAuthorized = $state(false);
  let authMode     = $state<'token' | 'browser'>('token');
  let apiToken     = $state('');
  let accountId    = $state('');
  let cancelled    = $state(false); // distinguish cancel from error

  // Sync when project changes; verify per-project cert on disk
  $effect(() => {
    if (project) {
      apiToken     = project.api_token  ?? '';
      accountId    = project.account_id ?? '';
      authMode     = project.auth_mode as 'token' | 'browser';
      cfAuthorized = project.browser_authed ?? false;
      if (project.auth_mode === 'browser') {
        api.checkCfAuth(project.id).then(v => { cfAuthorized = v; });
      }
    }
  });

  async function save() {
    saving = true;
    const updated = { ...project, auth_mode: authMode, api_token: apiToken, account_id: accountId };
    await api.upsertProject(updated);
    store.upsertProject(updated);
    saving = false;
  }

  async function authorize() {
    cancelled = false;
    store.status = 'processing';
    store.clearLogs();
    try {
      await api.cloudflaredLogin(project.id);
      cfAuthorized = true;
      store.status = 'idle';
      const updated = { ...project, auth_mode: 'browser' as const, browser_authed: true };
      await api.upsertProject(updated);
      store.upsertProject(updated);
    } catch (e) {
      if (cancelled) {
        store.status = 'idle'; // user chose to cancel — not an error
      } else {
        store.status = 'failed';
        store.appendLog(`FATAL: ${e}`);
      }
    }
  }

  async function cancelLogin() {
    cancelled = true;
    await api.cancelLogin();
    store.status = 'idle';
    store.clearLogs();
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

  function cancel() {
    store.selectedId = null;
  }
</script>

<div class="flex-1 overflow-y-auto p-5">
  <div class="max-w-lg mx-auto space-y-4">

    <!-- Heading -->
    <div class="flex items-start justify-between">
      <div>
        <h2 class="text-sm font-semibold">{project.domain}</h2>
        <p class="text-xs text-base-content/40 mt-0.5">
          Set up authentication to manage tunnels for this domain.
        </p>
      </div>
      <!-- Cancel / deselect — always clickable, even during auth -->
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
              onclick={() => (authMode = m as 'token' | 'browser')}
              disabled={store.busy}
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
                bind:value={apiToken} disabled={store.busy}
                class="input input-sm w-full bg-base-100 border-base-300
                       focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50
                       font-mono text-[11px]" />
            </div>
            <div class="space-y-1.5">
              <p class="text-xs font-medium text-base-content/50">Account ID</p>
              <input type="text" placeholder="a1b2c3d4…"
                bind:value={accountId} disabled={store.busy}
                class="input input-sm w-full bg-base-100 border-base-300
                       focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50
                       font-mono text-[11px]" />
            </div>
          </div>
          <p class="text-[11px] text-base-content/35">
            Token needs <span class="font-mono text-base-content/50">Tunnel:Edit</span>
            and <span class="font-mono text-base-content/50">DNS:Edit</span> permissions.
          </p>
          <div class="flex justify-end">
            <button
              class="btn btn-primary btn-sm"
              onclick={save}
              disabled={!apiToken || !accountId || store.busy || saving}
            >
              {#if saving}<span class="loading loading-spinner loading-xs"></span>{/if}
              Save &amp; Continue
            </button>
          </div>

        <!-- ── Browser / cloudflared ── -->
        {:else}
          {#if store.cloudflaredFound === false}
            <!-- cloudflared not installed -->
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
            <!-- Authorization in progress -->
            <div class="rounded-lg bg-base-300/50 border border-amber-400/20 px-3 py-3 space-y-2.5">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2.5">
                  <span class="loading loading-spinner loading-xs text-amber-400 flex-shrink-0"></span>
                  <span class="text-xs font-medium text-base-content/70">Waiting for browser authorization…</span>
                </div>
                <button
                  class="btn btn-ghost btn-xs text-red-400/60 hover:text-red-400"
                  onclick={cancelLogin}
                >Cancel login</button>
              </div>
              <p class="text-[11px] text-base-content/40 pl-5">
                If the browser didn't open, click the URL in the terminal log below to open it.
              </p>
            </div>

          {:else if cfAuthorized}
            <!-- Already authorized -->
            <div class="flex items-center gap-3 rounded-lg bg-base-300/50 border border-base-300 px-3 py-2.5">
              <span class="w-2 h-2 rounded-full bg-emerald-400 flex-shrink-0"></span>
              <span class="text-xs text-base-content/60 flex-1">
                Authorized ·
                <span class="font-mono text-[10px] text-base-content/30">
                  dockflare-{project.id.slice(0, 8)}….pem
                </span>
              </span>
              <button class="btn btn-xs btn-ghost text-base-content/40 hover:text-base-content/70"
                      onclick={authorize}>
                Reopen browser
              </button>
            </div>
            <div class="flex justify-end">
              <button class="btn btn-primary btn-sm" onclick={save} disabled={saving}>
                {#if saving}<span class="loading loading-spinner loading-xs"></span>{/if}
                Continue
              </button>
            </div>

          {:else}
            <!-- Not yet authorized -->
            <div class="flex items-center gap-3 rounded-lg bg-base-300/50 border border-base-300 px-3 py-2.5">
              <span class="w-2 h-2 rounded-full bg-amber-400 flex-shrink-0"></span>
              <span class="text-xs text-base-content/50 flex-1">Not yet authorized</span>
              <button class="btn btn-xs btn-primary" onclick={authorize}>
                Authorize with Cloudflare
              </button>
            </div>
            {#if store.status === 'failed'}
              <!-- Previous attempt failed — offer retry -->
              <div class="flex items-center justify-between">
                <span class="text-[11px] text-red-400/80">Authorization failed — check the log below.</span>
                <button class="btn btn-ghost btn-xs text-base-content/50" onclick={authorize}>
                  Try again
                </button>
              </div>
            {/if}
          {/if}

          <p class="text-[11px] text-base-content/35">
            Opens a browser window to log in to Cloudflare. No API token required.
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
        disabled={store.busy}
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
