import type { AppStatus, Project, TunnelInfo } from './types';

class AppStore {
  // ── Appearance ────────────────────────────────────────────
  theme = $state<'dark' | 'light'>('dark');

  // ── Projects ──────────────────────────────────────────────
  projects    = $state<Project[]>([]);
  selectedId  = $state<string | null>(null);

  get selectedProject(): Project | null {
    return this.projects.find(p => p.id === this.selectedId) ?? null;
  }

  // ── Tunnels for selected project ──────────────────────────
  tunnels        = $state<TunnelInfo[]>([]);
  tunnelsLoading = $state(false);

  // ── cloudflared binary check (global, not per-project) ───────
  cloudflaredFound = $state<boolean | null>(null);

  // ── Operation status ──────────────────────────────────────
  status = $state<AppStatus>('idle');
  logs   = $state<string[]>([]);

  get busy() { return this.status === 'processing'; }

  // ── Theme ─────────────────────────────────────────────────
  toggleTheme() {
    this.theme = this.theme === 'dark' ? 'light' : 'dark';
  }

  // ── Projects ──────────────────────────────────────────────
  upsertProject(p: Project) {
    const i = this.projects.findIndex(x => x.id === p.id);
    this.projects = i >= 0
      ? this.projects.map((x, j) => (j === i ? p : x))
      : [...this.projects, p];
  }

  removeProject(id: string) {
    this.projects = this.projects.filter(p => p.id !== id);
    if (this.selectedId === id) this.selectedId = null;
  }

  // ── Logs ──────────────────────────────────────────────────
  appendLog(line: string) { this.logs = [...this.logs, line]; }
  clearLogs()              { this.logs = []; }
}

export const store = new AppStore();
