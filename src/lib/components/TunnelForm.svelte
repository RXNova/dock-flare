<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import { domainError } from '$lib/utils';
  import type { TunnelConfig } from '$lib/types';

  let { onSuccess, onCancel }: { onSuccess: () => void; onCancel: () => void } = $props();

  let form = $state<TunnelConfig>({
    tunnel_name:     '',
    public_hostname: '',
    k8s_namespace:   'default',
    internal_service:'',
  });

  const hostnameErr = $derived(domainError(form.public_hostname));
  const ready       = $derived(
    !!(form.tunnel_name && form.public_hostname && !hostnameErr
       && form.k8s_namespace && form.internal_service)
  );

  async function deploy() {
    const project = store.selectedProject!;
    store.status = 'processing';
    store.clearLogs();
    try {
      await api.deployTunnel(project, form);
      store.status = 'success';
      // Refresh tunnel list
      store.tunnels = await api.listProjectTunnels(project);
      onSuccess();
    } catch (e) {
      store.status = 'failed';
      store.appendLog(`FATAL: ${e}`);
    }
  }
</script>

<div class="rounded-xl border border-primary/20 bg-primary/5 overflow-hidden mb-3">
  <div class="px-4 pt-3.5 pb-3 border-b border-primary/15 flex items-center justify-between">
    <span class="text-[10px] font-semibold uppercase tracking-widest text-primary/60">
      New tunnel
    </span>
    <button
      class="text-[10px] text-base-content/40 hover:text-base-content/70 transition-colors"
      onclick={onCancel}
      disabled={store.busy}
    >Cancel</button>
  </div>

  <div class="p-4">
    <div class="grid grid-cols-2 gap-3">
      <div class="space-y-1.5">
        <p class="text-xs font-medium text-base-content/50">Tunnel Name</p>
        <input type="text" placeholder="my-tunnel"
          bind:value={form.tunnel_name} disabled={store.busy}
          class="input input-sm w-full bg-base-100 border-base-300
                 focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50" />
      </div>

      <div class="space-y-1.5">
        <div class="flex items-baseline justify-between">
          <p class="text-xs font-medium text-base-content/50">Public Hostname</p>
          {#if hostnameErr}
            <span class="text-[10px] text-red-400">{hostnameErr}</span>
          {/if}
        </div>
        <input type="text" placeholder="app.example.com"
          bind:value={form.public_hostname} disabled={store.busy}
          class="input input-sm w-full bg-base-100
                 focus:outline-none focus:ring-2
                 {hostnameErr
                   ? 'border-red-500 focus:ring-red-500/25'
                   : 'border-base-300 focus:ring-primary/25 focus:border-primary/50'}" />
      </div>

      <div class="space-y-1.5">
        <p class="text-xs font-medium text-base-content/50">K8s Namespace</p>
        <input type="text" placeholder="default"
          bind:value={form.k8s_namespace} disabled={store.busy}
          class="input input-sm w-full bg-base-100 border-base-300
                 focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50" />
      </div>

      <div class="space-y-1.5">
        <p class="text-xs font-medium text-base-content/50">Internal Service</p>
        <input type="text" placeholder="http://svc:8080"
          bind:value={form.internal_service} disabled={store.busy}
          class="input input-sm w-full bg-base-100 border-base-300 font-mono text-[11px]
                 focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50" />
      </div>
    </div>

    <div class="flex justify-end mt-4">
      <button
        class="btn btn-primary btn-sm gap-2"
        onclick={deploy}
        disabled={!ready || store.busy}
      >
        {#if store.busy}
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
</div>
