use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct OrchestrationConfig {
    pub auth_mode: String, // "token" | "browser"
    #[serde(default)]
    pub api_token: String,
    #[serde(default)]
    pub account_id: String,
    pub tunnel_name: String,
    pub public_domain: String,
    pub k8s_namespace: String,
    pub internal_service: String,
}

// ── Cloudflare API types ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CfResponse<T> {
    result: Option<T>,
    success: bool,
    errors: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct TunnelResult {
    id: String,
}

#[derive(Deserialize)]
struct ZoneResult {
    id: String,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn emit_log(app: &AppHandle, msg: &str) {
    let _ = app.emit("log", msg.to_string());
}

fn cf_home() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cloudflared")
}

async fn check_binary(name: &str) -> Result<(), String> {
    let out = tokio::process::Command::new("which")
        .arg(name)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("'{}' not found in PATH", name))
    }
}

// Run a command silently and return trimmed stdout.
async fn run_command(prog: &str, args: &[&str]) -> Result<String, String> {
    let out = tokio::process::Command::new(prog)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("exec '{}': {}", prog, e))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

// Spawn a command and stream every stdout/stderr line as a log event.
async fn stream_command(app: &AppHandle, prog: &str, args: &[&str]) -> Result<(), String> {
    let mut child = tokio::process::Command::new(prog)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn '{}': {}", prog, e))?;

    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());
    let a1 = app.clone();
    let a2 = app.clone();
    let t1 = tokio::spawn(async move {
        let mut lines = stdout.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            emit_log(&a1, &line);
        }
    });
    let t2 = tokio::spawn(async move {
        let mut lines = stderr.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            emit_log(&a2, &line);
        }
    });

    let status = child.wait().await.map_err(|e| e.to_string())?;
    let _ = tokio::join!(t1, t2);

    if !status.success() {
        Err(format!("'{}' exited non-zero", prog))
    } else {
        Ok(())
    }
}

fn extract_zone(domain: &str) -> String {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() >= 2 { parts[parts.len() - 2..].join(".") } else { domain.to_string() }
}

// ── Auth-check commands ───────────────────────────────────────────────────────

#[tauri::command]
fn check_cf_auth() -> bool {
    cf_home().join("cert.pem").exists()
}

#[tauri::command]
async fn check_cloudflared() -> bool {
    check_binary("cloudflared").await.is_ok()
}

#[tauri::command]
#[allow(dead_code)]
async fn install_cloudflared(app: AppHandle) -> Result<(), String> {
    if let Err(e) = check_binary("brew").await {
        let msg = format!("Homebrew not found — install it from https://brew.sh then run: brew install cloudflared ({})", e);
        emit_log(&app, &format!("ERROR: {}", msg));
        return Err(msg);
    }
    emit_log(&app, "Running: brew install cloudflared");
    stream_command(&app, "brew", &["install", "cloudflared"]).await
}

// ── Browser-login command ─────────────────────────────────────────────────────

#[tauri::command]
async fn cloudflared_login(app: AppHandle) -> Result<(), String> {
    if let Err(e) = check_binary("cloudflared").await {
        emit_log(&app, &format!("ERROR: {}", e));
        return Err(e);
    }
    emit_log(&app, "Opening Cloudflare authorization in your browser...");
    emit_log(&app, "(If the browser doesn't open, copy the URL printed below.)");
    stream_command(&app, "cloudflared", &["tunnel", "login"]).await?;
    emit_log(&app, "Authorization complete.");
    Ok(())
}

// ── Cloudflare REST API helpers (token-auth path) ─────────────────────────────

async fn create_cf_tunnel(
    client: &Client,
    config: &OrchestrationConfig,
    secret: &str,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Req<'a> { name: &'a str, tunnel_secret: &'a str }
    let resp = client
        .post(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel",
            config.account_id
        ))
        .bearer_auth(&config.api_token)
        .json(&Req { name: &config.tunnel_name, tunnel_secret: secret })
        .send().await.map_err(|e| e.to_string())?;
    let body: CfResponse<TunnelResult> = resp.json().await.map_err(|e| e.to_string())?;
    if !body.success { return Err(format!("CF error: {:?}", body.errors)); }
    Ok(body.result.ok_or("empty tunnel result")?.id)
}

async fn get_tunnel_token(
    client: &Client,
    config: &OrchestrationConfig,
    tunnel_id: &str,
) -> Result<String, String> {
    let resp = client
        .get(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel/{}/token",
            config.account_id, tunnel_id
        ))
        .bearer_auth(&config.api_token)
        .send().await.map_err(|e| e.to_string())?;
    let body: CfResponse<String> = resp.json().await.map_err(|e| e.to_string())?;
    if !body.success { return Err(format!("Token error: {:?}", body.errors)); }
    body.result.ok_or_else(|| "no token".to_string())
}

async fn configure_tunnel_ingress(
    client: &Client,
    config: &OrchestrationConfig,
    tunnel_id: &str,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Rule { #[serde(skip_serializing_if = "Option::is_none")] hostname: Option<String>, service: String }
    #[derive(Serialize)]
    struct Cfg { ingress: Vec<Rule> }
    #[derive(Serialize)]
    struct Body { config: Cfg }
    let resp = client
        .put(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel/{}/configurations",
            config.account_id, tunnel_id
        ))
        .bearer_auth(&config.api_token)
        .json(&Body { config: Cfg { ingress: vec![
            Rule { hostname: Some(config.public_domain.clone()), service: config.internal_service.clone() },
            Rule { hostname: None, service: "http_status:404".to_string() },
        ]}})
        .send().await.map_err(|e| e.to_string())?;
    let body: CfResponse<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
    if !body.success { return Err(format!("Ingress config error: {:?}", body.errors)); }
    Ok(())
}

async fn get_zone_id(client: &Client, config: &OrchestrationConfig, zone: &str) -> Result<String, String> {
    let resp = client
        .get("https://api.cloudflare.com/client/v4/zones")
        .bearer_auth(&config.api_token)
        .query(&[("name", zone)])
        .send().await.map_err(|e| e.to_string())?;
    let body: CfResponse<Vec<ZoneResult>> = resp.json().await.map_err(|e| e.to_string())?;
    if !body.success { return Err(format!("Zone error: {:?}", body.errors)); }
    body.result.ok_or_else(|| "no zones".to_string())?
        .into_iter().next().map(|z| z.id)
        .ok_or_else(|| format!("no zone for '{}'", zone))
}

async fn create_dns_record(client: &Client, config: &OrchestrationConfig, tunnel_id: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Rec { r#type: String, name: String, content: String, ttl: u32, proxied: bool }
    let zone_id = get_zone_id(client, config, &extract_zone(&config.public_domain)).await?;
    let resp = client
        .post(format!("https://api.cloudflare.com/client/v4/zones/{}/dns_records", zone_id))
        .bearer_auth(&config.api_token)
        .json(&Rec {
            r#type: "CNAME".to_string(), name: config.public_domain.clone(),
            content: format!("{}.cfargotunnel.com", tunnel_id), ttl: 1, proxied: true,
        })
        .send().await.map_err(|e| e.to_string())?;
    let body: CfResponse<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
    if !body.success { return Err(format!("DNS error: {:?}", body.errors)); }
    Ok(())
}

async fn deploy_helm(app: &AppHandle, config: &OrchestrationConfig, token: &str) -> Result<(), String> {
    emit_log(app, "  -> helm repo add cloudflare...");
    run_command("helm", &["repo", "add", "cloudflare", "https://cloudflare.github.io/helm-charts"]).await.ok();
    emit_log(app, "  -> helm repo update...");
    run_command("helm", &["repo", "update"]).await.map_err(|e| format!("helm repo update: {}", e))?;
    emit_log(app, "  -> helm upgrade --install cloudflared...");
    stream_command(app, "helm", &[
        "upgrade", "--install", "cloudflared", "cloudflare/cloudflare-tunnel",
        "--namespace", &config.k8s_namespace, "--create-namespace",
        "--set", &format!("cloudflare.tunnelToken={}", token),
    ]).await.map_err(|e| format!("helm deploy: {}", e))
}

// ── Browser-auth orchestration path ──────────────────────────────────────────

async fn orchestrate_browser(app: &AppHandle, config: &OrchestrationConfig) -> Result<(), String> {
    emit_log(app, "[1/5] Checking prerequisites...");
    for bin in &["cloudflared", "kubectl", "helm"] {
        if let Err(e) = check_binary(bin).await {
            emit_log(app, &format!("ERROR: {}", e));
            return Err(e);
        }
    }
    if !cf_home().join("cert.pem").exists() {
        let msg = "Not authorized — use 'Authorize with Cloudflare' first";
        emit_log(app, &format!("ERROR: {}", msg));
        return Err(msg.to_string());
    }
    emit_log(app, "  -> prerequisites OK");

    emit_log(app, "[2/5] Creating tunnel via cloudflared CLI...");
    let create_out = run_command("cloudflared", &["tunnel", "create", &config.tunnel_name])
        .await.map_err(|e| { emit_log(app, &format!("ERROR: {}", e)); e })?;
    // Output: "Created tunnel <name> with id <uuid>"
    let tunnel_id = create_out.lines()
        .find(|l| l.contains("with id "))
        .and_then(|l| l.split("with id ").nth(1))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "Could not parse tunnel ID from cloudflared output".to_string())?;
    emit_log(app, &format!("  -> Tunnel ID: {}", tunnel_id));

    emit_log(app, "[3/5] Creating DNS record via cloudflared CLI...");
    if let Err(e) = stream_command(app, "cloudflared", &["tunnel", "route", "dns", &config.tunnel_name, &config.public_domain]).await {
        emit_log(app, &format!("ERROR: {}", e));
        emit_log(app, "  -> Cleaning up: deleting tunnel to avoid orphan...");
        match run_command("cloudflared", &["tunnel", "delete", "-f", &config.tunnel_name]).await {
            Ok(_)  => emit_log(app, "  -> Tunnel deleted."),
            Err(ce) => emit_log(app, &format!("  -> Could not delete tunnel: {}", ce)),
        }
        let msg = format!(
            "DNS routing failed for '{}'. Make sure the hostname belongs to the zone you authorized. Tunnel has been deleted.",
            config.public_domain
        );
        return Err(msg);
    }

    emit_log(app, "[4/5] Generating K8s manifests...");
    let creds_path = cf_home().join(format!("{}.json", tunnel_id));
    let creds_bytes = std::fs::read(&creds_path)
        .map_err(|e| format!("Cannot read credentials {}: {}", creds_path.display(), e))?;
    let creds_b64 = STANDARD.encode(&creds_bytes);

    let manifest = format!(
r#"---
apiVersion: v1
kind: Namespace
metadata:
  name: {ns}
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: cloudflared-config
  namespace: {ns}
data:
  config.yaml: |
    tunnel: {tunnel_id}
    credentials-file: /etc/cloudflared/creds/credentials.json
    no-autoupdate: true
    ingress:
      - hostname: {domain}
        service: {service}
      - service: http_status:404
---
apiVersion: v1
kind: Secret
metadata:
  name: cloudflared-creds
  namespace: {ns}
type: Opaque
data:
  credentials.json: {creds_b64}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cloudflared
  namespace: {ns}
spec:
  replicas: 2
  selector:
    matchLabels:
      app: cloudflared
  template:
    metadata:
      labels:
        app: cloudflared
    spec:
      containers:
      - name: cloudflared
        image: cloudflare/cloudflared:latest
        args: ["tunnel", "--config", "/etc/cloudflared/config/config.yaml", "run"]
        volumeMounts:
        - name: config
          mountPath: /etc/cloudflared/config
          readOnly: true
        - name: creds
          mountPath: /etc/cloudflared/creds
          readOnly: true
      volumes:
      - name: config
        configMap:
          name: cloudflared-config
      - name: creds
        secret:
          secretName: cloudflared-creds
"#,
        ns = config.k8s_namespace,
        tunnel_id = tunnel_id,
        domain = config.public_domain,
        service = config.internal_service,
        creds_b64 = creds_b64,
    );

    let manifest_path = std::env::temp_dir().join("cloudflared-manifest.yaml");
    std::fs::write(&manifest_path, &manifest)
        .map_err(|e| format!("write manifest: {}", e))?;
    emit_log(app, &format!("  -> manifest written to {}", manifest_path.display()));

    emit_log(app, "[5/5] Applying K8s manifests via kubectl...");
    stream_command(app, "kubectl", &["apply", "-f", manifest_path.to_str().unwrap()])
        .await.map_err(|e| { emit_log(app, &format!("ERROR: {}", e)); e })?;

    Ok(())
}

// ── Token-auth orchestration path ─────────────────────────────────────────────

async fn orchestrate_token(app: &AppHandle, config: &OrchestrationConfig) -> Result<(), String> {
    emit_log(app, "[1/6] Checking prerequisites and validating domain...");
    for bin in &["kubectl", "helm"] {
        if let Err(e) = check_binary(bin).await {
            emit_log(app, &format!("ERROR: {}", e));
            return Err(e);
        }
    }
    let client = Client::new();
    let zone_name = extract_zone(&config.public_domain);
    emit_log(app, &format!("  -> Checking zone '{}'...", zone_name));
    match get_zone_id(&client, config, &zone_name).await {
        Ok(_)  => emit_log(app, &format!("  -> Zone found. kubectl and helm OK.")),
        Err(_) => {
            let msg = format!(
                "'{}' is not in your Cloudflare account. Check the hostname is on a zone your API token can access.",
                config.public_domain
            );
            emit_log(app, &format!("ERROR: {}", msg));
            return Err(msg);
        }
    }

    let mut secret_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret_bytes);
    let tunnel_secret = STANDARD.encode(secret_bytes);

    emit_log(app, "[2/6] Creating Cloudflare tunnel...");
    let tunnel_id = match create_cf_tunnel(&client, config, &tunnel_secret).await {
        Ok(id) => id,
        Err(e) => { emit_log(app, &format!("ERROR: {}", e)); return Err(e); }
    };
    emit_log(app, &format!("  -> Tunnel ID: {}", tunnel_id));

    emit_log(app, "[3/6] Fetching tunnel token...");
    let tunnel_token = match get_tunnel_token(&client, config, &tunnel_id).await {
        Ok(t) => t,
        Err(e) => { emit_log(app, &format!("ERROR: {}", e)); return Err(e); }
    };
    emit_log(app, "  -> Token acquired");

    emit_log(app, "[4/6] Configuring tunnel ingress rules...");
    if let Err(e) = configure_tunnel_ingress(&client, config, &tunnel_id).await {
        emit_log(app, &format!("ERROR: {}", e)); return Err(e);
    }
    emit_log(app, &format!("  -> {} -> {}", config.public_domain, config.internal_service));

    emit_log(app, "[5/6] Creating DNS CNAME record...");
    if let Err(e) = create_dns_record(&client, config, &tunnel_id).await {
        emit_log(app, &format!("ERROR: {}", e)); return Err(e);
    }
    emit_log(app, &format!("  -> {} -> {}.cfargotunnel.com", config.public_domain, tunnel_id));

    emit_log(app, "[6/6] Deploying cloudflared via Helm...");
    if let Err(e) = deploy_helm(app, config, &tunnel_token).await {
        emit_log(app, &format!("ERROR: {}", e)); return Err(e);
    }
    emit_log(app, "  -> Helm chart deployed");

    Ok(())
}

// ── Main orchestrate command ──────────────────────────────────────────────────

#[tauri::command]
async fn orchestrate(app: AppHandle, config: OrchestrationConfig) -> Result<(), String> {
    emit_log(&app, &format!("=== DockFlare Orchestration [{}] ===", config.auth_mode));
    let result = if config.auth_mode == "browser" {
        orchestrate_browser(&app, &config).await
    } else {
        orchestrate_token(&app, &config).await
    };
    if result.is_ok() {
        emit_log(&app, "=== Complete! ===");
    }
    result
}

// ── Tauri entry ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            orchestrate,
            cloudflared_login,
            check_cf_auth,
            check_cloudflared,
            install_cloudflared
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
