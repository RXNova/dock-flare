import { invoke } from '@tauri-apps/api/core';
import type { Project, TunnelConfig, TunnelInfo } from './types';

export const api = {
  // Projects
  getProjects:   ()              => invoke<Project[]>('get_projects'),
  upsertProject: (p: Project)    => invoke<void>('upsert_project', { project: p }),
  deleteProject: (id: string)    => invoke<void>('delete_project', { id }),

  // Tunnels
  listProjectTunnels: (p: Project) =>
    invoke<TunnelInfo[]>('list_project_tunnels', { project: p }),

  deployTunnel: (p: Project, t: TunnelConfig) =>
    invoke<void>('deploy_tunnel', { project: p, tunnel: t }),

  teardownTunnel: (p: Project, tunnelName: string, hostname: string, namespace: string) =>
    invoke<void>('teardown_tunnel', { project: p, tunnelName, hostname, namespace }),

  // cloudflared — auth is per-project (each gets its own cert file)
  checkCloudflared:   ()                   => invoke<boolean>('check_cloudflared'),
  checkCfAuth:        (projectId: string)  => invoke<boolean>('check_cf_auth', { projectId }),
  cloudflaredLogin:   (projectId: string)  => invoke<void>('cloudflared_login', { projectId }),
  cancelLogin:        ()                   => invoke<void>('cancel_login'),
  installCloudflared: ()                   => invoke<void>('install_cloudflared'),
  openUrl:            (url: string)        => invoke<void>('open_url', { url }),
};
