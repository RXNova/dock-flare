<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import type { TunnelInfo } from '$lib/types';
  import { Check, Copy, ExternalLink, Pencil, Trash2, X } from '@lucide/svelte';

  let { tunnel }: { tunnel: TunnelInfo } = $props();

  let confirming  = $state(false);
  let pendingNs   = $state('');
  let editing     = $state(false);
  let editService = $state('');
  let copied      = $state(false);

  function copyHostname() {
    navigator.clipboard.writeText(tunnel.hostname ?? '');
    copied = true;
    setTimeout(() => { copied = false; }, 1500);
  }

  $effect(() => { if (editing)    editService = tunnel.service  ?? ''; });
  $effect(() => { if (confirming) pendingNs   = tunnel.namespace ?? 'default'; });

  async function saveEdit() {
    const project = store.selectedProject!;
    try {
      await api.updateTunnelService(project, tunnel.name, editService);
      editing = false;
      store.tunnels = await api.listProjectTunnels(project);
    } catch (e) {
      store.appendLog(`Edit failed: ${e}`);
    }
  }

  async function teardown() {
    const project = store.selectedProject!;
    confirming = false;
    store.status = 'processing';
    store.clearLogs();
    try {
      await api.teardownTunnel(project, tunnel.name, tunnel.hostname ?? '', pendingNs);
      store.status = 'success';
      store.tunnels = await api.listProjectTunnels(project);
    } catch (e) {
      store.status = 'failed';
      store.appendLog(`FATAL: ${e}`);
    }
  }

  const statusDot: Record<string, string> = {
    healthy:  'bg-emerald-400',
    degraded: 'bg-amber-400',
    down:     'bg-red-400',
    inactive: 'bg-zinc-500',
    unknown:  'bg-zinc-600',
    active:   'bg-emerald-400',
  };
  const statusColor: Record<string, string> = {
    healthy: 'text-emerald-400',
    active:  'text-emerald-400',
    degraded:'text-amber-400',
    down:    'text-red-400',
  };
  const dot   = $derived(statusDot[tunnel.status]   ?? 'bg-zinc-600');
  const scol  = $derived(statusColor[tunnel.status] ?? 'text-zinc-400');
  const isToken = $derived(store.selectedProject?.auth_mode === 'token');
</script>

<div class="px-4 py-2.5 border-b border-base-300/60 last:border-0
            hover:bg-base-300/15 transition-colors space-y-1.5">

  <!-- Row 1: name · type · status · actions -->
  <div class="flex items-center gap-2">
    <span class="w-1.5 h-1.5 rounded-full flex-shrink-0 {dot}"></span>

    <span class="text-xs font-semibold truncate flex-1 min-w-0">{tunnel.name}</span>

    {#if tunnel.target_type}
      <span class="badge badge-xs flex-shrink-0
                   {tunnel.target_type === 'local' ? 'badge-info' : 'badge-warning'} opacity-80">
        {tunnel.target_type}
      </span>
    {/if}

    <span class="text-[10px] font-medium flex-shrink-0 {scol}">{tunnel.status}</span>

    <!-- Actions -->
    {#if confirming}
      <div class="flex items-center gap-1 ml-auto">
        <span class="text-[10px] text-base-content/40 mr-1">ns:</span>
        <input type="text" bind:value={pendingNs}
          class="input input-xs w-24 bg-base-100 border-base-300 font-mono text-[10px]
                 focus:outline-none focus:ring-1 focus:ring-red-500/30"
          placeholder="namespace" />
        <button class="btn btn-error btn-xs px-2" onclick={teardown} disabled={store.busy}>
          {#if store.busy}
            <span class="loading loading-spinner loading-xs"></span>
          {:else}
            <Check class="w-3 h-3" strokeWidth={2.5} />
          {/if}
        </button>
        <button class="btn btn-ghost btn-xs px-1" onclick={() => (confirming = false)} disabled={store.busy}>
          <X class="w-3 h-3" strokeWidth={2} />
        </button>
      </div>
    {:else}
      <div class="flex items-center gap-0.5 ml-1 flex-shrink-0">
        {#if isToken}
          <button class="btn btn-ghost btn-xs px-1.5 text-base-content/30 hover:text-primary hover:bg-primary/10"
                  onclick={() => (editing = !editing)} disabled={store.busy} title="Edit service URL">
            <Pencil class="w-3 h-3" strokeWidth={2} />
          </button>
        {/if}
        <button class="btn btn-ghost btn-xs px-1.5 text-base-content/30 hover:text-red-400 hover:bg-red-400/10"
                onclick={() => (confirming = true)} disabled={store.busy} title="Tear down tunnel">
          <Trash2 class="w-3 h-3" strokeWidth={2} />
        </button>
      </div>
    {/if}
  </div>

  <!-- Row 2: hostname · service · namespace -->
  <div class="flex items-center gap-3 pl-3.5 flex-wrap">

    <!-- Hostname -->
    <div class="flex items-center gap-1 min-w-0">
      <span class="text-[10px] text-base-content/35 font-medium flex-shrink-0">host</span>
      <span class="text-[10px] font-mono text-base-content/60 truncate max-w-[180px]">
        {tunnel.hostname ?? '—'}
      </span>
      {#if tunnel.hostname}
        <button onclick={copyHostname} title={copied ? 'Copied!' : 'Copy'} class="flex-shrink-0">
          {#if copied}
            <Check class="w-3 h-3 text-emerald-400" strokeWidth={2.5} />
          {:else}
            <Copy class="w-3 h-3 text-base-content/30 hover:text-base-content/70 transition-colors" strokeWidth={2} />
          {/if}
        </button>
        <button onclick={() => api.openUrl('https://' + tunnel.hostname)}
                title="Open in browser" class="flex-shrink-0">
          <ExternalLink class="w-3 h-3 text-base-content/30 hover:text-base-content/70 transition-colors" strokeWidth={2} />
        </button>
      {/if}
    </div>

    <!-- Divider -->
    <span class="text-base-content/15 text-[10px] flex-shrink-0">·</span>

    <!-- Service (editable) -->
    <div class="flex items-center gap-1 min-w-0">
      <span class="text-[10px] text-base-content/35 font-medium flex-shrink-0">svc</span>
      {#if editing}
        <input type="text" bind:value={editService}
          class="input input-xs w-40 bg-base-100 border-base-300 font-mono text-[10px]
                 focus:outline-none focus:ring-1 focus:ring-primary/30" />
        <button class="btn btn-xs btn-primary px-1.5" onclick={saveEdit}>
          <Check class="w-3 h-3" strokeWidth={2.5} />
        </button>
        <button class="btn btn-ghost btn-xs px-1" onclick={() => (editing = false)}>
          <X class="w-3 h-3" strokeWidth={2} />
        </button>
      {:else}
        <span class="text-[10px] font-mono text-base-content/50 truncate max-w-[140px]">
          {tunnel.service ?? '—'}
        </span>
      {/if}
    </div>

    <!-- Divider -->
    <span class="text-base-content/15 text-[10px] flex-shrink-0">·</span>

    <!-- Namespace -->
    <div class="flex items-center gap-1">
      <span class="text-[10px] text-base-content/35 font-medium flex-shrink-0">ns</span>
      <span class="text-[10px] font-mono text-base-content/50">{tunnel.namespace ?? '—'}</span>
    </div>
  </div>
</div>
