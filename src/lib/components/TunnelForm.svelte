<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import { domainError } from '$lib/utils';
  import type { TunnelConfig } from '$lib/types';
  import { Zap } from '@lucide/svelte';

  let { onSuccess, onCancel }: { onSuccess: () => void; onCancel: () => void } = $props();

  let subdomain = $state('');

  let form = $state<TunnelConfig>({
    tunnel_name:     '',
    public_hostname: '',
    target_type:     'local',
    k8s_namespace:   'default',
    internal_service:'',
  });

  const fullHostname = $derived(subdomain ? `${subdomain}.${store.selectedProject?.domain}` : store.selectedProject?.domain || '');
  const hostnameErr = $derived(domainError(fullHostname));

  const nameErr = $derived(
    form.tunnel_name && !/^[a-z0-9][a-z0-9-]*$/.test(form.tunnel_name)
      ? 'Lowercase letters, numbers, and hyphens only'
      : null
  );

  const serviceErr = $derived(
    form.internal_service && !/^https?:\/\/.+/.test(form.internal_service)
      ? 'Must start with http:// or https://'
      : null
  );

  const ready = $derived(
    !!(form.tunnel_name && !nameErr && subdomain && !hostnameErr
       && form.internal_service && !serviceErr
       && (form.target_type === 'local' || form.k8s_namespace))
  );

  async function deploy() {
    const project = store.selectedProject!;
    form.public_hostname = fullHostname;
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
        <div class="flex items-baseline justify-between mb-1.5">
          <p class="text-xs font-medium text-base-content/50">Tunnel Name</p>
          {#if nameErr}
            <span class="text-[10px] text-red-400">{nameErr}</span>
          {/if}
        </div>
        <input type="text" placeholder="my-tunnel"
          bind:value={form.tunnel_name} disabled={store.busy}
          class="input input-sm w-full bg-base-100
                 focus:outline-none focus:ring-2
                 {nameErr
                   ? 'border-red-500 focus:ring-red-500/25'
                   : 'border-base-300 focus:ring-primary/25 focus:border-primary/50'}" />
      </div>

      <div class="space-y-1.5">
        <div class="flex items-baseline justify-between mb-1.5">
          <p class="text-xs font-medium text-base-content/50">Subdomain</p>
          {#if hostnameErr}
            <span class="text-[10px] text-red-400">{hostnameErr}</span>
          {/if}
        </div>
        <div class="flex items-center">
          <input type="text" placeholder="app"
            bind:value={subdomain} disabled={store.busy}
            class="input input-sm w-full bg-base-100 rounded-r-none border-r-0
                   focus:outline-none focus:ring-2 focus:z-10
                   {hostnameErr
                     ? 'border-red-500 focus:ring-red-500/25'
                     : 'border-base-300 focus:ring-primary/25 focus:border-primary/50'}" />
          <div class="px-3 bg-base-200 border border-base-300 rounded-r-lg h-8 flex items-center text-xs text-base-content/50 whitespace-nowrap">
            .{store.selectedProject?.domain}
          </div>
        </div>
      </div>

      <div class="space-y-1.5">
        <p class="text-xs font-medium text-base-content/50">Target Strategy</p>
        <select
          bind:value={form.target_type} disabled={store.busy}
          class="select select-sm w-full bg-base-100 border-base-300
                 focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50"
        >
          <option value="local">Local Webserver</option>
          <option value="k8s">Kubernetes</option>
        </select>
      </div>

      {#if form.target_type === 'k8s'}
        <div class="space-y-1.5">
          <p class="text-xs font-medium text-base-content/50">K8s Namespace</p>
          <input type="text" placeholder="default"
            bind:value={form.k8s_namespace} disabled={store.busy}
            class="input input-sm w-full bg-base-100 border-base-300
                   focus:outline-none focus:ring-2 focus:ring-primary/25 focus:border-primary/50" />
        </div>
      {/if}

      <div class="space-y-1.5">
        <div class="flex items-baseline justify-between mb-1.5">
          <p class="text-xs font-medium text-base-content/50">Internal Service</p>
          {#if serviceErr}
            <span class="text-[10px] text-red-400">{serviceErr}</span>
          {/if}
        </div>
        <input type="text" placeholder={form.target_type === 'local' ? "http://localhost:3000" : "http://svc:8080"}
          bind:value={form.internal_service} disabled={store.busy}
          class="input input-sm w-full bg-base-100 font-mono text-[11px]
                 focus:outline-none focus:ring-2
                 {serviceErr
                   ? 'border-red-500 focus:ring-red-500/25'
                   : 'border-base-300 focus:ring-primary/25 focus:border-primary/50'}" />
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
          <Zap class="w-3.5 h-3.5" strokeWidth={2} />
          Deploy Tunnel
        {/if}
      </button>
    </div>
  </div>
</div>
