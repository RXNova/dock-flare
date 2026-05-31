<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import { store } from '$lib/store.svelte';
  import { api } from '$lib/api';
  import Header from '$lib/components/Header.svelte';
  import ProjectSidebar from '$lib/components/ProjectSidebar.svelte';
  import ProjectView from '$lib/components/ProjectView.svelte';
  import TerminalLog from '$lib/components/TerminalLog.svelte';

  let unlisten: (() => void) | null = null;

  $effect(() => {
    document.documentElement.setAttribute('data-theme', store.theme);
    localStorage.setItem('df-theme', store.theme);
  });

  onMount(async () => {
    // Restore theme
    const savedTheme = localStorage.getItem('df-theme') as 'dark' | 'light' | null;
    if (savedTheme) store.theme = savedTheme;

    // Load projects from disk
    store.projects = await api.getProjects();

    // Check cloudflared binary (shared; per-project cert checked in AuthSetup)
    store.cloudflaredFound = await api.checkCloudflared();

    // Stream log events from Rust
    unlisten = await listen<string>('log', (e) => {
      store.appendLog(e.payload);
    });
  });

  onDestroy(() => unlisten?.());
</script>

<div class="h-screen bg-base-100 flex flex-col overflow-hidden select-none">
  <Header />

  <div class="flex flex-1 min-h-0">
    <ProjectSidebar />
    <ProjectView />
  </div>

  <TerminalLog />
</div>
