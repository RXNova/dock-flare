<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { statusCfg } from '$lib/utils';
  import { Cloud, Sun, Moon } from '@lucide/svelte';
</script>

<header class="flex-shrink-0 px-5 py-3.5 border-b border-base-300
               flex items-center justify-between">
  <div class="flex items-center gap-3">
    <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0">
      <Cloud class="w-4 h-4 text-primary" strokeWidth={1.75} />
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
        {#if store.status === 'processing'}
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full opacity-60
                       {statusCfg.processing.dot}"></span>
        {/if}
        <span class="relative inline-flex h-2 w-2 rounded-full {statusCfg[store.status].dot}"></span>
      </span>
      <span class="text-[11px] font-medium {statusCfg[store.status].text}">
        {statusCfg[store.status].label}
      </span>
    </div>

    <!-- Theme toggle -->
    <button
      class="w-8 h-8 rounded-lg flex items-center justify-center
             text-base-content/50 hover:text-base-content hover:bg-base-200 transition-colors"
      onclick={() => store.toggleTheme()}
      title="Toggle theme"
    >
      {#if store.theme === 'dark'}
        <Sun class="w-4 h-4" strokeWidth={1.75} />
      {:else}
        <Moon class="w-4 h-4" strokeWidth={1.75} />
      {/if}
    </button>
  </div>
</header>
