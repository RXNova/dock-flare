# DockFlare

A macOS desktop app that provisions [Cloudflare Tunnels](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) and deploys them into a local Kubernetes cluster — no port forwarding, no public IP required.

Built with **Tauri v2**, **Rust**, and **SvelteKit**.

---

## What it does

1. **Authenticates** with Cloudflare — via API token or browser OAuth (`cloudflared tunnel login`)
2. **Creates a Named Tunnel** on your Cloudflare account
3. **Configures ingress rules** mapping a public hostname to an internal Kubernetes service
4. **Creates a DNS CNAME** pointing your hostname to the tunnel
5. **Deploys `cloudflared`** into your cluster via Helm (API token path) or `kubectl apply` (browser auth path)

Traffic flow:
```
Internet → Cloudflare Edge → cloudflared pod (in-cluster) → internal service
```

---

## Prerequisites

| Tool | Purpose |
|------|---------|
| `kubectl` | Kubernetes cluster access |
| `helm` | Deploy cloudflared chart (API token path) |
| `cloudflared` | Browser auth + tunnel management (browser path) |

---

## Development

```bash
# Install dependencies
npm install

# Run in dev mode (hot-reload)
npm run tauri dev

# Build for production
npm run tauri build
```

**Requirements:** Rust (stable), Node.js ≥ 18

---

## Stack

| Layer | Technology |
|-------|-----------|
| UI | SvelteKit 2, Tailwind CSS v4, DaisyUI v5 |
| Desktop shell | Tauri v2 |
| Backend logic | Rust (tokio, reqwest, serde) |
| Cloudflare API | REST v4 (`/cfdtunnel`, `/dns_records`) |
| K8s deployment | Helm (`cloudflare/cloudflare-tunnel`) or kubectl manifests |
