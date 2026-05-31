<script lang="ts">
  import { store } from '$lib/store.svelte';
  import { statusCfg } from '$lib/utils';
</script>

<header class="flex-shrink-0 px-5 py-3.5 border-b border-base-300
               flex items-center justify-between">
  <div class="flex items-center gap-3">
    <div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0">
      <svg class="w-4 h-4 text-primary" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
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
        <!-- Sun icon -->
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z" />
        </svg>
      {:else}
        <!-- Moon icon -->
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.72 9.72 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" />
        </svg>
      {/if}
    </button>
  </div>
</header>
