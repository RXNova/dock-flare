<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  type AuthMode  = 'token' | 'browser';
  type Status    = 'idle' | 'processing' | 'success' | 'failed';
  type ActiveTab = 'deploy' | 'manage';

  interface TunnelInfo { id: string; name: string; status: string; }

  let theme            = $state<'dark' | 'light'>('dark');
  let activeTab        = $state<ActiveTab>('deploy');
  let step             = $state<1 | 2>(1);
  let authMode         = $state<AuthMode>('token');
  let status           = $state<Status>('idle');
  let cfAuthorized     = $state(false);
  let cloudflaredFound = $state<boolean | null>(null);
  let logs             = $state<string[]>([]);
  let logEl            = $state<HTMLDivElement | null>(null);

  let config = $state({
    api_token:        '',
    account_id:       '',
    tunnel_name:      '',
    public_domain:    '',
    k8s_namespace:    'default',
    internal_service: '',
  });

  // Manage tab
  let tunnels         = $state<TunnelInfo[]>([]);
  let tunnelsLoading  = $state(false);
  let confirmTeardown = $state(false);
  let teardownForm    = $state({ tunnel_name: '', public_domain: '', k8s_namespace: 'default' });

  let unlisten: (() => void) | null = null;

  // ── Persistence ────────────────────────────────────────────────────────────

  $effect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('df-theme', theme);
  });

  $effect(() => { localStorage.setItem('df-auth-mode', authMode); });
  $effect(() => { localStorage.setItem('df-config', JSON.stringify(config)); });

  async function refreshChecks() {
    cloudflaredFound = await invoke<boolean>('check_cloudflared');
    if (cloudflaredFound) cfAuthorized = await invoke<boolean>('check_cf_auth');
  }

  onMount(async () => {
    const savedTheme = localStorage.getItem('df-theme') as 'dark' | 'light' | null;
    if (savedTheme) theme = savedTheme;

    const savedMode = localStorage.getItem('df-auth-mode') as AuthMode | null;
    if (savedMode) authMode = savedMode;

    const savedConfig = localStorage.getItem('df-config');
    if (savedConfig) {
      try { config = { ...config, ...JSON.parse(savedConfig) }; } catch {}
    }

    await refreshChecks();
    if (cfAuthorized) step = 2;

    unlisten = await listen<string>('log', (e) => {
      logs = [...logs, e.payload];
      setTimeout(() => { if (logEl) logEl.scrollTop = logEl.scrollHeight; }, 0);
    });
  });
  onDestroy(() => unlisten?.());

  const busy = $derived(status === 'processing');

  // ── Auth readiness ─────────────────────────────────────────────────────────

  const tokenAuthReady   = $derived(!!(config.api_token && config.account_id));
  const browserAuthReady = $derived(cfAuthorized && cloudflaredFound === true);
  const authReady        = $derived(authMode === 'token' ? tokenAuthReady : browserAuthReady);

  // ── Tunnel config validation ───────────────────────────────────────────────

  function domainError(d: string): string | null {
    if (!d.trim()) return null;
    if (d.includes(' ')) return 'No spaces allowed';
    const p = d.split('.');
    if (p.length < 2 || p.some(s => s.length === 0)) return 'Enter a full hostname';
    if (!/^[a-zA-Z0-9][a-zA-Z0-9.\-]*[a-zA-Z0-9]$/.test(d)) return 'Invalid characters';
    return null;
  }
  const domainErr   = $derived(domainError(config.public_domain));
  const tunnelReady = $derived(
    !!(config.tunnel_name && config.public_domain && !domainErr
       && config.k8s_namespace && config.internal_service)
  );

  // ── Teardown readiness ─────────────────────────────────────────────────────

  const teardownReady = $derived(
    authMode === 'token'
      ? !!(teardownForm.tunnel_name && teardownForm.public_domain && teardownForm.k8s_namespace
           && config.api_token && config.account_id)
      : !!(teardownForm.tunnel_name && teardownForm.k8s_namespace)
  );

  // ── Actions ────────────────────────────────────────────────────────────────

  async function run(action: () => Promise<void>) {
    status = 'processing';
    try {
      await action();
      status = 'idle';
    } catch (e) {
      status = 'failed';
      logs = [...logs, `FATAL: ${e}`];
    }
  }

  const installCloudflared = () => run(async () => {
    await invoke('install_cloudflared');
    await refreshChecks();
  });

  const authorize = () => run(async () => {
    logs = [];
    await invoke('cloudflared_login');
    cfAuthorized = true;
  });

  async function deploy() {
    logs = [];
    status = 'processing';
    try {
      await invoke('orchestrate', { config: { auth_mode: authMode, ...config } });
      status = 'success';
    } catch (e) {
      status = 'failed';
      logs = [...logs, `FATAL: ${e}`];
    }
  }

  async function fetchTunnels() {
    if (!config.api_token || !config.account_id) return;
    tunnelsLoading = true;
    try {
      tunnels = await invoke<TunnelInfo[]>('list_tunnels', {
        apiToken: config.api_token,
        accountId: config.account_id,
      });
    } catch (e) {
      logs = [...logs, `ERROR listing tunnels: ${e}`];
    } finally {
      tunnelsLoading = false;
    }
  }

  function openManageTab() {
    if (busy) return;
    activeTab = 'manage';
    // Pre-fill teardown form from deploy config on first open
    if (!teardownForm.tunnel_name) teardownForm.tunnel_name = config.tunnel_name;
    if (!teardownForm.public_domain) teardownForm.public_domain = config.public_domain;
    if (!teardownForm.k8s_namespace || teardownForm.k8s_namespace === 'default')
      teardownForm.k8s_namespace = config.k8s_namespace;
    if (authMode === 'token' && config.api_token && config.account_id) fetchTunnels();
  }

  async function executeTeardown() {
    logs = [];
    confirmTeardown = false;
    status = 'processing';
    try {
      await invoke('teardown', {
        config: {
          auth_mode:        authMode,
          api_token:        config.api_token,
          account_id:       config.account_id,
          tunnel_name:      teardownForm.tunnel_name,
          public_domain:    teardownForm.public_domain,
          k8s_namespace:    teardownForm.k8s_namespace,
          internal_service: '',
        },
      });
      status = 'success';
      if (authMode === 'token' && config.api_token && config.account_id) await fetchTunnels();
    } catch (e) {
      status = 'failed';
      logs = [...logs, `FATAL: ${e}`];
    }
  }

  // ── Terminal colouring ─────────────────────────────────────────────────────

  function lineStyle(line: string): string {
    if (/error|fatal/i.test(line))         return 'text-red-400';
    if (line.startsWith('==='))            return 'text-white font-semibold';
    if (/^\[\d+\/\d+\]/.test(line))        return 'text-sky-400';
    if (/complete|success|\bOK\b|found|acquired|written|deployed/i.test(line))
                                           return 'text-emerald-400';
    if (line.trimStart().startsWith('->')) return 'text-zinc-300';
    return 'text-zinc-200';
  }

  const statusCfg: Record<Status, { dot: string; label: string; text: string }> = {
    idle:       { dot: 'bg-zinc-500',    label: 'Idle',      text: 'text-zinc-400'    },
    processing: { dot: 'bg-amber-400',   label: 'Deploying', text: 'text-amber-400'   },
    success:    { dot: 'bg-emerald-400', label: 'Success',   text: 'text-emerald-400' },
    failed:     { dot: 'bg-red-400',     label: 'Failed',    text: 'text-red-400'     },
  };

  const authSummary = $derived(authMode === 'token' ? 'API token' : 'Cloudflare browser login');
</script>

<div class="h-screen bg-base-100 flex flex-col overflow-hidden select-none">

  <!-- ── Header ─────────────────────────────────────────────── -->
  <header class="flex-shrink-0 px-5 py-3.5 border-b border-base-300
                 flex items-center justify-between">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0">
        <svg class="w-4 h-4 text-primary" xmlns="http://www.w3.org/2000/svg"
             viewBox="0 0 24 24" fill="currentColor">
          <path d="M4.5 9.75a6 6 0 0111.573-2.226 3.75 3.75 0 014.133 4.303A4.5 4.5 0
                   0118 20.25H6.75a5.25 5.25 0 01-2.23-10.004 6.072 6.072 0 01-.02-.496z"/>
        </svg>
      </div>
      <div>
        <p class="text-sm font-semibold leading-none tracking-tight">DockFlare</p>
        <p class="text-[11px] text-base-content/35 mt-0.5">Cloudflare tunnels → local Kubernetes</p>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <!-- Status pill -->
      <div class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-base-200 border border-base-300">
        <span class="relative flex h-2 w-2">
          {#if status === 'processing'}
            <span class="animate-ping absolute inline-flex h-full w-full rounded-full
                         opacity-60 {statusCfg.processing.dot}"></span>
          {/if}
          <span class="relative inline-flex h-2 w-2 rounded-full {statusCfg[status].dot}"></span>
        </span>
        <span class="text-[11px] font-medium {statusCfg[status].text}">{statusCfg[status].label}</span>
      </div>

      <!-- Theme toggle -->
      <button
        class="w-7 h-7 rounded-lg flex items-center justify-center
               text-base-content/40 hover:text-base-content/70 hover:bg-base-200 transition-colors"
        onclick={() => (theme = theme === 'dark' ? 'light' : 'dark')}
        title={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
      >
        {#if theme === 'dark'}
          <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 2.25a.75.75 0 01.75.75v2.25a.75.75 0 01-1.5 0V3a.75.75 0 01.75-.75zM7.5 12a4.5 4.5 0 119 0 4.5 4.5 0 01-9 0zM18.894 6.166a.75.75 0 00-1.06-1.06l-1.591 1.59a.75.75 0 101.06 1.061l1.591-1.59zM21.75 12a.75.75 0 01-.75.75h-2.25a.75.75 0 010-1.5H21a.75.75 0 01.75.75zM17.834 18.894a.75.75 0 001.06-1.06l-1.59-1.591a.75.75 0 10-1.061 1.06l1.59 1.591zM12 18a.75.75 0 01.75.75V21a.75.75 0 01-1.5 0v-2.25A.75.75 0 0112 18zM7.166 17.834a.75.75 0 00-1.06 1.06l1.59 1.591a.75.75 0 001.061-1.06l-1.59-1.591zM6 12a.75.75 0 01-.75.75H3a.75.75 0 010-1.5h2.25A.75.75 0 016 12zM6.166 6.166a.75.75 0 001.06-1.06L5.634 3.515a.75.75 0 10-1.06 1.06l1.59 1.591z"/>
          </svg>
        {:else}
          <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path fill-rule="evenodd" clip-rule="evenodd"
              d="M9.528 1.718a.75.75 0 01.162.819A8.97 8.97 0 009 6a9 9 0 009 9 8.97 8.97 0 003.463-.69.75.75 0 01.981.98 10.503 10.503 0 01-9.694 6.46c-5.799 0-10.5-4.701-10.5-10.5 0-4.368 2.667-8.112 6.46-9.694a.75.75 0 01.818.162z"/>
          </svg>
        {/if}
      </button>
    </div>
  </header>

  <!-- ── Body ───────────────────────────────────────────────── -->
  <div class="flex-1 overflow-y-auto px-5 py-4 space-y-3 min-h-0">

    <!-- Tab bar -->
    <div class="flex bg-base-200 rounded-xl border border-base-300 p-1 gap-1">
      <button
        class="flex-1 py-1.5 rounded-lg text-xs font-medium transition-all duration-150
               {activeTab === 'deploy'
                 ? 'bg-base-100 shadow-sm text-base-content'
                 : 'text-base-content/40 hover:text-base-content/60'}"
        onclick={() => { if (!busy) activeTab = 'deploy'; }}
        disabled={busy}
      >Deploy</button>
      <button
        class="flex-1 py-1.5 rounded-lg text-xs font-medium transition-all duration-150
               {activeTab === 'manage'
                 ? 'bg-base-100 shadow-sm text-base-content'
                 : 'text-base-content/40 hover:text-base-content/60'}"
        onclick={openManageTab}
        disabled={busy}
      >Manage</button>
    </div>

    <!-- ══ Deploy tab ══ -->
    {#if activeTab === 'deploy'}

      <!-- Step indicator -->
      <div class="flex items-center gap-2">
        <button
          class="flex items-center gap-2 group"
          onclick={() => { if (!busy) step = 1; }}
          disabled={busy}
        >
          <span class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold
                       transition-colors
                       {step === 1
                         ? 'bg-primary text-primary-content'
                         : 'bg-base-300 text-base-content/40 group-hover:text-base-content/60'}">
            {step > 1 ? '✓' : '1'}
          </span>
          <span class="text-xs font-medium transition-colors
                       {step === 1 ? 'text-base-content' : 'text-base-content/40 group-hover:text-base-content/60'}">
            Authentication
          </span>
        </button>
        <div class="flex-1 h-px bg-base-300 max-w-8"></div>
        <span class="flex items-center gap-2">
          <span class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold
                       transition-colors
                       {step === 2 ? 'bg-primary text-primary-content' : 'bg-base-300 text-base-content/40'}">
            2
          </span>
          <span class="text-xs font-medium transition-colors
                       {step === 2 ? 'text-base-content' : 'text-base-content/40'}">
            Tunnel Setup
          </span>
        </span>
      </div>

      <!-- Step 1: Authentication -->
      {#if step === 1}
        <div class="rounded-xl border border-base-300 bg-base-200 overflow-hidden">
          <div class="px-4 pt-3.5 pb-3 border-b border-base-300 flex items-center justify-between">
            <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
              Choose authentication method
            </span>
            <div class="flex bg-base-300/60 rounded-lg p-0.5 gap-px">
              {#each [['token','API Token'],['browser','Browser Login']] as [mode, label] (mode)}
                <button
                  class="px-3.5 py-1 rounded-[6px] text-xs font-medium transition-all duration-150
                         disabled:opacity-40
                         {authMode === mode
                           ? 'bg-base-100 shadow-sm text-base-content'
                           : 'text-base-content/40 hover:text-base-content/60'}"
                  onclick={() => (authMode = mode as AuthMode)}
                  disabled={busy}
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
                    bind:value={config.api_token} disabled={busy}
                    class="input input-sm w-full bg-base-100 border-base-300
                           focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50
                           font-mono text-[11px]" />
                </div>
                <div class="space-y-1.5">
                  <p class="text-xs font-medium text-base-content/50">Account ID</p>
                  <input type="text" placeholder="a1b2c3d4…"
                    bind:value={config.account_id} disabled={busy}
                    class="input input-sm w-full bg-base-100 border-base-300
                           focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50
                           font-mono text-[11px]" />
                </div>
              </div>
              <p class="text-[11px] text-base-content/35">
                Your API token needs <span class="font-mono text-base-content/50">Tunnel:Edit</span>
                and <span class="font-mono text-base-content/50">DNS:Edit</span> permissions.
              </p>
            {/if}

            {#if authMode === 'browser'}
              <div class="space-y-3">
                {#if cloudflaredFound === false}
                  <div class="flex items-center gap-3 rounded-lg bg-base-300/50 border border-base-300 px-3 py-2.5">
                    <span class="w-2 h-2 rounded-full bg-red-400 flex-shrink-0"></span>
                    <span class="text-xs text-base-content/50 flex-1">
                      <code class="font-mono text-base-content/70">cloudflared</code> not found —
                      <code class="font-mono text-base-content/40 text-[10px]">brew install cloudflared</code>
                    </span>
                    <button class="btn btn-xs btn-primary" onclick={installCloudflared} disabled={busy}>
                      {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
                      Install via Homebrew
                    </button>
                  </div>
                {:else if cfAuthorized}
                  <div class="flex items-center gap-3 rounded-lg bg-base-300/50 border border-base-300 px-3 py-2.5">
                    <span class="w-2 h-2 rounded-full bg-emerald-400 flex-shrink-0"></span>
                    <span class="text-xs text-base-content/60 flex-1">
                      Authorized ·
                      <span class="font-mono text-base-content/30 text-[10px]">~/.cloudflared/cert.pem</span>
                    </span>
                    <button class="btn btn-xs btn-ghost text-base-content/30 hover:text-base-content/60"
                            onclick={authorize} disabled={busy}>
                      Re-authorize
                    </button>
                  </div>
                {:else}
                  <div class="flex items-center gap-3 rounded-lg bg-base-300/50 border border-base-300 px-3 py-2.5">
                    <span class="w-2 h-2 rounded-full bg-amber-400 flex-shrink-0"></span>
                    <span class="text-xs text-base-content/50 flex-1">Not yet authorized</span>
                    <button class="btn btn-xs btn-primary" onclick={authorize} disabled={busy}>
                      {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
                      Authorize with Cloudflare
                    </button>
                  </div>
                {/if}
                <p class="text-[11px] text-base-content/35">
                  Opens a browser window to log in to your Cloudflare account. No API token needed.
                </p>
              </div>
            {/if}
          </div>

          <div class="px-4 pb-4 flex justify-end">
            <button
              class="btn btn-primary btn-sm gap-1.5"
              onclick={() => (step = 2)}
              disabled={!authReady || busy}
            >
              Continue
              <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
                <path fill-rule="evenodd" clip-rule="evenodd"
                  d="M12.97 3.97a.75.75 0 011.06 0l7.5 7.5a.75.75 0 010 1.06l-7.5 7.5a.75.75 0
                     01-1.06-1.06l6.22-6.22H3a.75.75 0 010-1.5h16.19l-6.22-6.22a.75.75 0 010-1.06z"/>
              </svg>
            </button>
          </div>
        </div>
      {/if}

      <!-- Step 2: Tunnel Setup -->
      {#if step === 2}
        <div class="rounded-xl border border-base-300 bg-base-200 overflow-hidden">
          <div class="px-4 pt-3.5 pb-3 border-b border-base-300 flex items-center justify-between">
            <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
              Tunnel configuration
            </span>
            <span class="flex items-center gap-1.5 text-[11px] text-base-content/50
                         bg-base-300/60 px-2.5 py-1 rounded-full">
              <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
              {authSummary}
            </span>
          </div>

          <div class="p-4">
            <div class="grid grid-cols-2 gap-3">
              <div class="space-y-1.5">
                <p class="text-xs font-medium text-base-content/50">Tunnel Name</p>
                <input type="text" placeholder="my-tunnel"
                  bind:value={config.tunnel_name} disabled={busy}
                  class="input input-sm w-full bg-base-100 border-base-300
                         focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50" />
              </div>

              <div class="space-y-1.5">
                <div class="flex items-baseline justify-between">
                  <p class="text-xs font-medium text-base-content/50">Public Hostname</p>
                  {#if domainErr}
                    <span class="text-[10px] text-red-400">{domainErr}</span>
                  {:else}
                    <span class="text-[10px] text-base-content/25">subdomain on authorized zone</span>
                  {/if}
                </div>
                <input type="text" placeholder="app.example.com"
                  bind:value={config.public_domain} disabled={busy}
                  class="input input-sm w-full bg-base-100
                         focus:outline-none focus:ring-2
                         {domainErr
                           ? 'border-red-500 focus:ring-red-500/25'
                           : 'border-base-300 focus:ring-primary/25 focus:border-primary/50'}" />
              </div>

              <div class="space-y-1.5">
                <p class="text-xs font-medium text-base-content/50">K8s Namespace</p>
                <input type="text" placeholder="default"
                  bind:value={config.k8s_namespace} disabled={busy}
                  class="input input-sm w-full bg-base-100 border-base-300
                         focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50" />
              </div>

              <div class="space-y-1.5">
                <p class="text-xs font-medium text-base-content/50">Internal Service</p>
                <input type="text" placeholder="http://svc:8080"
                  bind:value={config.internal_service} disabled={busy}
                  class="input input-sm w-full bg-base-100 border-base-300
                         focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50
                         font-mono text-[11px]" />
              </div>
            </div>
          </div>

          <div class="px-4 pb-4 flex items-center justify-between">
            <button
              class="btn btn-ghost btn-sm gap-1.5 text-base-content/40 hover:text-base-content/70"
              onclick={() => (step = 1)}
              disabled={busy}
            >
              <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
                <path fill-rule="evenodd" clip-rule="evenodd"
                  d="M11.03 3.97a.75.75 0 010 1.06l-6.22 6.22H21a.75.75 0 010 1.5H4.81l6.22 6.22a.75.75 0
                     11-1.06 1.06l-7.5-7.5a.75.75 0 010-1.06l7.5-7.5a.75.75 0 011.06 0z"/>
              </svg>
              Back
            </button>

            <button
              class="btn btn-primary btn-sm gap-2"
              onclick={deploy}
              disabled={!tunnelReady || busy}
            >
              {#if busy}
                <span class="loading loading-spinner loading-xs"></span>
                Deploying…
              {:else}
                <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
                  <path fill-rule="evenodd" clip-rule="evenodd"
                    d="M14.615 1.595a.75.75 0 01.359.852L12.982 9.75h7.268a.75.75 0 01.548 1.262l-10.5
                       11.25a.75.75 0 01-1.272-.71l1.992-7.302H3.75a.75.75 0 01-.548-1.262l10.5-11.25a.75.75
                       0 01.913-.143z"/>
                </svg>
                Deploy Tunnel
              {/if}
            </button>
          </div>
        </div>
      {/if}

    {/if}

    <!-- ══ Manage tab ══ -->
    {#if activeTab === 'manage'}
      <div class="rounded-xl border border-base-300 bg-base-200 overflow-hidden">
        <div class="px-4 pt-3.5 pb-3 border-b border-base-300 flex items-center justify-between">
          <span class="text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
            Tear down tunnel
          </span>
          <span class="flex items-center gap-1.5 text-[11px] text-base-content/50
                       bg-base-300/60 px-2.5 py-1 rounded-full">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
            {authSummary}
          </span>
        </div>

        <!-- Token mode: live tunnel list -->
        {#if authMode === 'token'}
          {#if !config.api_token || !config.account_id}
            <div class="px-4 py-3 border-b border-base-300">
              <p class="text-xs text-base-content/40">
                Set API token and account ID in the Deploy tab to list tunnels.
              </p>
            </div>
          {:else}
            <div class="px-4 pt-3 pb-2 border-b border-base-300">
              <div class="flex items-center justify-between mb-2">
                <p class="text-[10px] font-semibold uppercase tracking-widest text-base-content/40">
                  Active tunnels
                </p>
                <button
                  class="btn btn-ghost btn-xs text-base-content/40 h-auto min-h-0 py-0.5"
                  onclick={fetchTunnels}
                  disabled={busy || tunnelsLoading}
                >
                  {#if tunnelsLoading}
                    <span class="loading loading-spinner loading-xs"></span>
                  {:else}
                    <svg class="w-3 h-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
                      <path fill-rule="evenodd" clip-rule="evenodd"
                        d="M4.755 10.059a7.5 7.5 0 0112.548-3.364l1.903 1.927H15a.75.75 0 000 1.5h4.5a.75.75
                           0 00.75-.75V4.5a.75.75 0 00-1.5 0v3.093L17.53 5.887a9 9 0 00-15.044 4.034.75.75
                           0 001.269.138zM19.245 13.941a.75.75 0 00-1.269-.138A7.5 7.5 0 016.427 17.167l-1.903-1.927H9a.75.75
                           0 000-1.5H4.5a.75.75 0 00-.75.75v4.5a.75.75 0 001.5 0v-3.093l1.72 1.706a9 9 0 0015.044-4.034.75.75
                           0 00-1.769-.438z"/>
                    </svg>
                  {/if}
                  Refresh
                </button>
              </div>

              {#if tunnels.length === 0 && !tunnelsLoading}
                <p class="text-xs text-base-content/30 py-1">No active tunnels found.</p>
              {:else}
                <div class="space-y-1">
                  {#each tunnels as tunnel (tunnel.id)}
                    <button
                      class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-left transition-colors
                             {teardownForm.tunnel_name === tunnel.name
                               ? 'bg-primary/10 border border-primary/30'
                               : 'hover:bg-base-300/60 border border-transparent'}"
                      onclick={() => { teardownForm.tunnel_name = tunnel.name; }}
                      disabled={busy}
                    >
                      <span class="w-2 h-2 rounded-full flex-shrink-0
                                   {tunnel.status === 'healthy' ? 'bg-emerald-400' : 'bg-zinc-500'}"></span>
                      <span class="text-xs font-medium flex-1 text-left">{tunnel.name}</span>
                      <span class="text-[10px] text-base-content/30 font-mono">{tunnel.id.slice(0, 8)}…</span>
                      <span class="text-[10px] {tunnel.status === 'healthy' ? 'text-emerald-400' : 'text-zinc-400'}">
                        {tunnel.status}
                      </span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        {/if}

        <!-- Teardown form -->
        <div class="p-4">
          <div class="grid grid-cols-2 gap-3">
            <div class="space-y-1.5">
              <p class="text-xs font-medium text-base-content/50">Tunnel Name</p>
              <input type="text" placeholder="my-tunnel"
                bind:value={teardownForm.tunnel_name} disabled={busy}
                class="input input-sm w-full bg-base-100 border-base-300
                       focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50" />
            </div>

            {#if authMode === 'token'}
              <div class="space-y-1.5">
                <p class="text-xs font-medium text-base-content/50">Public Domain</p>
                <input type="text" placeholder="app.example.com"
                  bind:value={teardownForm.public_domain} disabled={busy}
                  class="input input-sm w-full bg-base-100 border-base-300
                         focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50" />
              </div>
            {/if}

            <div class="space-y-1.5">
              <p class="text-xs font-medium text-base-content/50">K8s Namespace</p>
              <input type="text" placeholder="default"
                bind:value={teardownForm.k8s_namespace} disabled={busy}
                class="input input-sm w-full bg-base-100 border-base-300
                       focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50" />
            </div>
          </div>
        </div>

        <!-- Action row -->
        <div class="px-4 pb-4 flex items-center justify-end gap-3">
          {#if confirmTeardown}
            <span class="text-[11px] text-red-400/80 mr-1">
              Deletes tunnel, DNS record, and K8s resources.
            </span>
            <button
              class="btn btn-error btn-sm gap-1.5"
              onclick={executeTeardown}
              disabled={busy}
            >
              {#if busy}<span class="loading loading-spinner loading-xs"></span>{/if}
              Confirm
            </button>
            <button
              class="btn btn-ghost btn-sm"
              onclick={() => (confirmTeardown = false)}
              disabled={busy}
            >Cancel</button>
          {:else}
            <button
              class="btn btn-error btn-outline btn-sm gap-1.5"
              onclick={() => (confirmTeardown = true)}
              disabled={!teardownReady || busy}
            >
              <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
                <path fill-rule="evenodd" clip-rule="evenodd"
                  d="M16.5 4.478v.227a48.816 48.816 0 013.878.512.75.75 0 11-.256 1.478l-.209-.035-1.005
                     13.07a3 3 0 01-2.991 2.77H8.084a3 3 0 01-2.991-2.77L4.087 6.66l-.209.035a.75.75
                     0 01-.256-1.478A48.567 48.567 0 017.5 4.705v-.227c0-1.564 1.213-2.9 2.816-2.951a52.662
                     52.662 0 013.369 0c1.603.051 2.815 1.387 2.815 2.951zm-6.136-1.452a51.196 51.196 0
                     013.273 0C14.39 3.05 15 3.684 15 4.478v.113a49.488 49.488 0 00-6 0v-.113c0-.794.609-1.428
                     1.364-1.452zm-.355 5.945a.75.75 0 10-1.5.058l.347 9a.75.75 0 101.499-.058l-.346-9zm5.48.058a.75.75
                     0 10-1.498-.058l-.347 9a.75.75 0 001.5.058l.345-9z"/>
              </svg>
              Tear Down
            </button>
          {/if}
        </div>
      </div>
    {/if}

    <!-- ── Terminal log ── -->
    <div class="rounded-xl border border-zinc-700 bg-zinc-950 overflow-hidden flex-shrink-0">
      <div class="flex items-center gap-2 px-3 py-2 border-b border-zinc-800">
        <span class="w-2.5 h-2.5 rounded-full bg-zinc-600"></span>
        <span class="w-2.5 h-2.5 rounded-full bg-zinc-600"></span>
        <span class="w-2.5 h-2.5 rounded-full bg-zinc-600"></span>
        <span class="ml-2 text-[10px] font-mono text-zinc-500">deployment log</span>
        {#if logs.length > 0}
          <button
            class="ml-auto text-[10px] text-zinc-500 hover:text-zinc-300 transition-colors"
            onclick={() => (logs = [])}
          >clear</button>
        {/if}
      </div>
      <div
        class="p-3 h-40 overflow-y-auto font-mono text-[11px] leading-[1.6]"
        bind:this={logEl}
      >
        {#if logs.length === 0}
          <span class="text-zinc-500">ready</span><span class="text-zinc-500 cursor-blink">█</span>
        {:else}
          {#each logs as line}
            <div class={lineStyle(line)}>{line}</div>
          {/each}
        {/if}
      </div>
    </div>

  </div>
</div>
