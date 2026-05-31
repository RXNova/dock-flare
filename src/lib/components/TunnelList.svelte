<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import TunnelRow from './TunnelRow.svelte';
  import TunnelForm from './TunnelForm.svelte';
  import { RefreshCw, Plus } from '@lucide/svelte';

  let project    = $derived(store.selectedProject!);
  let showForm   = $state(false);
  let loading    = $state(false);  // local — avoids class $state async reactivity issues

  async function refresh(force = false) {
    if (!force && store.tunnels.length > 0) return;
    loading = true;
    try {
      store.tunnels = await api.listProjectTunnels(project);
    } catch (e) {
      store.appendLog(`Error listing tunnels: ${e instanceof Error ? e.message : JSON.stringify(e)}`);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const id = store.selectedId;
    if (id) { store.tunnels = []; refresh(); }
  });

  const authBadge = $derived(project.auth_mode === 'token' ? 'API token' : 'cloudflared');

  async function reauth() {
    const updated = { ...project, browser_authed: false };
    await api.upsertProject(updated);
    store.upsertProject(updated);
  }

  async function reconfig() {
    await api.reconfigureProject(project.id);
    store.upsertProject({ ...project, api_token: '', account_id: '', domain: '' });
  }
</script>

<div class="flex-1 overflow-y-auto p-5 space-y-3">
  <!-- Project header -->
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-sm font-semibold">{project.name || project.domain}</h2>
      <div class="flex items-center gap-1.5 mt-0.5">
        <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
        {#if project.domain}
          <span class="text-[11px] font-mono text-base-content/50">{project.domain}</span>
          <span class="text-base-content/20">·</span>
        {/if}
        <span class="text-[11px] text-base-content/40">{authBadge}</span>
        {#if project.auth_mode === 'browser'}
          <span class="text-base-content/20">·</span>
          <button class="text-[11px] text-base-content/30 hover:text-base-content/60 transition-colors"
                  onclick={reauth} disabled={store.busy}>re-auth</button>
        {:else}
          <span class="text-base-content/20">·</span>
          <button class="text-[11px] text-base-content/30 hover:text-base-content/60 transition-colors"
                  onclick={reconfig} disabled={store.busy}>re-configure</button>
        {/if}
      </div>
    </div>
    <div class="flex items-center gap-2">
      <!-- Refresh -->
      <button
        class="btn btn-ghost btn-sm gap-1.5"
        onclick={() => refresh(true)}
        disabled={loading}
      >
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" strokeWidth={2} />
        Refresh
      </button>
      <!-- Add tunnel -->
      <button
        class="btn btn-primary btn-sm gap-1.5"
        onclick={() => (showForm = !showForm)}
        disabled={store.busy}
      >
        {#if showForm}
          Cancel
        {:else}
          <Plus class="w-3.5 h-3.5" strokeWidth={2} />
          Add Tunnel
        {/if}
      </button>
    </div>
  </div>

  <!-- Create form -->
  {#if showForm}
    <TunnelForm onSuccess={() => (showForm = false)} onCancel={() => (showForm = false)} />
  {/if}

  <!-- Tunnel list -->
  <div class="rounded-xl border border-base-300 bg-base-200 overflow-hidden">
    {#if loading && store.tunnels.length === 0}
      <div class="flex items-center justify-center gap-2 py-8 text-base-content/30">
        <span class="loading loading-spinner loading-sm"></span>
        <span class="text-xs">Loading tunnels…</span>
      </div>
    {:else if store.tunnels.length === 0}
      <div class="flex flex-col items-center justify-center py-10 gap-2">
        <p class="text-sm text-base-content/30">No tunnels yet</p>
        <button class="btn btn-ghost btn-xs text-base-content/40 gap-1"
                onclick={() => (showForm = true)}>
          <Plus class="w-3 h-3" strokeWidth={2} />
          Add your first tunnel
        </button>
      </div>
    {:else}
      {#each store.tunnels as tunnel (tunnel.id)}
        <TunnelRow {tunnel} />
      {/each}
    {/if}
  </div>
</div>
