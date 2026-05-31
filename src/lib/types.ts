export interface Project {
  id: string;
  name: string;            // user label, set at creation
  domain: string;          // auto-discovered zone; empty until authed
  auth_mode: 'token' | 'browser';
  api_token: string;
  account_id: string;
  browser_authed: boolean;
}

export interface ZoneInfo {
  zone: string;            // primary zone = the project's domain
  all: string[];           // all reachable zones (token mode may have several)
  account_id: string;      // discovered account ID (empty for browser mode)
}

export interface TunnelInfo {
  id: string;
  name: string;
  status: string;
  hostname: string | null;
  service: string | null;
  namespace: string | null;
  target_type: string | null;
}

export interface TunnelConfig {
  tunnel_name: string;
  public_hostname: string;
  target_type: 'k8s' | 'local';
  k8s_namespace: string;
  internal_service: string;
}

export type AppStatus = 'idle' | 'processing' | 'success' | 'failed';
