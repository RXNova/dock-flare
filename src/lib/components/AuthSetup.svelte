<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';

  let project = $derived(store.selectedProject!);
  let saving  = $state(false);

  // Local editable copies of credentials
  let apiToken   = $state('');
  let accountId  = $state('');
  let authMode   = $state<'token' | 'browser'>('token');

  // Sync when project changes
  $effect(() => {
    if (project) {
      apiToken  = project.api_token  ?? '';
      accountId = project.account_id ?? '';
      authMode  = project.auth_mode;
    }
  });

  async function save() {
    saving = true;
    const updated = { ...project, auth_mode: authMode, api_token: apiToken, account_id: accountId };
    await api.upsertProject(updated);
    store.upsertProject(updated);
    saving = false;
  }

  const authorize = async () => {
    store.status = 'processing';
    store.clearLogs();
    try {
      await api.cloudflaredLogin();
      store.cfAuthorized = true;
      store.status = 'idle';
      // Switch auth mode + save
      const updated = { ...project, auth_mode: 'browser' as const };
      await api.upsertProject(updated);
      store.upsertProject(updated);
    } catch (e) {
      store.status = 'failed';
      store.appendLog(`FATAL: ${e}`);
    }
  };

  const installCloudflared = async () => {
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
  };
</script>

<div class="flex-1 overflow-y-auto p-5">
  <div class="max-w-lg mx-auto space-y-4">
    <div>
      <h2 class="text-sm font-semibold">{project.domain}</h2>
      <p class="text-xs text-base-content/40 mt-0.5">
        Set up authentication to start managing tunnels for this domain.
      </p>
    </div>

    <!-- Mode switcher -->
    <div class="rounded-xl border border-base-300 bg-base-200 overflow-hidden">
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

        {:else}
          <!-- Browser / cloudflared auth -->
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
          {:else if store.cfAuthorized}
            <div class="flex items-center gap-3 rounded-lg bg-base-300/50 border border-base-300 px-3 py-2.5">
              <span class="w-2 h-2 rounded-full bg-emerald-400 flex-shrink-0"></span>
              <span class="text-xs text-base-content/60 flex-1">
                Authorized · <span class="font-mono text-[10px] text-base-content/30">~/.cloudflared/cert.pem</span>
              </span>
              <button class="btn btn-xs btn-ghost text-base-content/40" onclick={authorize} disabled={store.busy}>
                Re-authorize
              </button>
            </div>
            <div class="flex justify-end">
              <button
                class="btn btn-primary btn-sm"
                onclick={save}
                disabled={store.busy || saving}
              >Save &amp; Continue</button>
            </div>
          {:else}
            <div class="flex items-center gap-3 rounded-lg bg-base-300/50 border border-base-300 px-3 py-2.5">
              <span class="w-2 h-2 rounded-full bg-amber-400 flex-shrink-0"></span>
              <span class="text-xs text-base-content/50 flex-1">Not yet authorized</span>
              <button class="btn btn-xs btn-primary" onclick={authorize} disabled={store.busy}>
                {#if store.busy}<span class="loading loading-spinner loading-xs"></span>{/if}
                Authorize with Cloudflare
              </button>
            </div>
          {/if}
          <p class="text-[11px] text-base-content/35">
            Opens a browser window to log in. No API token required.
          </p>
        {/if}
      </div>
    </div>
  </div>
</div>
