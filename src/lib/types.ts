export interface Project {
  id: string;
  domain: string;
  auth_mode: 'token' | 'browser';
  api_token: string;
  account_id: string;
}

export interface TunnelInfo {
  id: string;
  name: string;
  status: string;
  hostname: string | null;
  service: string | null;
  namespace: string | null;
}

export interface TunnelConfig {
  tunnel_name: string;
  public_hostname: string;
  k8s_namespace: string;
  internal_service: string;
}

export type AppStatus = 'idle' | 'processing' | 'success' | 'failed';
