use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use reqwest::Client;
use std::process::Stdio;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::cloudflare;
use crate::config::{Project, TunnelConfig};

pub fn emit(app: &AppHandle, msg: &str) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| {
            let s = d.as_secs();
            format!("{:02}:{:02}:{:02}", (s % 86400) / 3600, (s % 3600) / 60, s % 60)
        })
        .unwrap_or_else(|_| "??:??:??".to_string());
    let _ = app.emit("log", format!("[{}] {}", ts, msg));
}

fn cf_home() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cloudflared")
}

pub fn cf_home_path() -> std::path::PathBuf {
    cf_home()
}

/// Per-project cert. cloudflared login always writes ~/.cloudflared/cert.pem;
/// we copy it here afterwards so each project keeps its own isolated credential.
pub fn project_cert_path(project_id: &str) -> std::path::PathBuf {
    cf_home().join(format!("dockflare-{}.pem", project_id))
}

pub async fn check_bin(name: &str) -> Result<(), String> {
    let ok = tokio::process::Command::new("which")
        .arg(name)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok { Ok(()) } else { Err(format!("'{}' not found in PATH", name)) }
}

pub async fn run_silent(prog: &str, args: &[&str]) -> Result<String, String> {
    let out = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::process::Command::new(prog).args(args).output(),
    )
    .await
    .map_err(|_| format!("'{}' timed out after 30s", prog))?
    .map_err(|e| format!("exec '{}': {}", prog, e))?;

    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub async fn run_streamed(app: &AppHandle, prog: &str, args: &[&str]) -> Result<(), String> {
    let mut child = tokio::process::Command::new(prog)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn '{}': {}", prog, e))?;

    let stdout = BufReader::new(
        child.stdout.take().ok_or_else(|| format!("failed to capture stdout from '{}'", prog))?,
    );
    let stderr = BufReader::new(
        child.stderr.take().ok_or_else(|| format!("failed to capture stderr from '{}'", prog))?,
    );
    let a1 = app.clone();
    let a2 = app.clone();
    let t1 = tokio::spawn(async move {
        let mut lines = stdout.lines();
        while let Ok(Some(l)) = lines.next_line().await { emit(&a1, &l); }
    });
    let t2 = tokio::spawn(async move {
        let mut lines = stderr.lines();
        while let Ok(Some(l)) = lines.next_line().await { emit(&a2, &l); }
    });
    let status = child.wait().await.map_err(|e| e.to_string())?;
    let _ = tokio::join!(t1, t2);
    if status.success() { Ok(()) } else { Err(format!("'{}' exited non-zero", prog)) }
}

// ── Token-auth deploy — returns (tunnel_id, local_pid) ───────────────────────

pub async fn deploy_token(
    app: &AppHandle,
    project: &Project,
    tunnel: &TunnelConfig,
) -> Result<(String, Option<u32>), String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    emit(app, "[1/7] Checking prerequisites...");
    let req_bins = if tunnel.target_type == "local" { vec!["cloudflared"] } else { vec!["kubectl", "helm"] };
    for bin in req_bins {
        check_bin(bin).await.map_err(|e| { emit(app, &format!("ERROR: {}", e)); e })?;
    }

    if tunnel.target_type != "local" {
        emit(app, "[2/7] Checking Kubernetes connectivity...");
        if run_silent("kubectl", &["cluster-info"]).await.is_err() {
            let msg = "Could not connect to Kubernetes cluster. Make sure Docker Desktop, Minikube, orbstack, or another local cluster is running and your KUBECONFIG is correct.";
            emit(app, &format!("ERROR: {}", msg));
            return Err(msg.to_string());
        }
    } else {
        emit(app, "[2/7] Skipping K8s (local process mode)");
    }

    let zone = cloudflare::extract_zone(&tunnel.public_hostname);
    emit(app, &format!("  -> Verifying zone '{}'...", zone));
    let zone_id = cloudflare::get_zone_id(&client, &project.api_token, &project.account_id, &zone)
        .await
        .map_err(|_| {
            let msg = format!(
                "'{}' not found in your Cloudflare account. Check the token has DNS access to this zone.",
                tunnel.public_hostname
            );
            emit(app, &format!("ERROR: {}", msg));
            msg
        })?;

    emit(app, "  -> Checking for duplicate tunnel name...");
    if let Ok(Some(id)) = cloudflare::find_tunnel_id(
        &client, &project.api_token, &project.account_id, &tunnel.tunnel_name,
    ).await {
        let msg = format!(
            "Tunnel '{}' (id: {}…) already exists — tear it down first or use a different name.",
            tunnel.tunnel_name, &id[..id.len().min(8)]
        );
        emit(app, &format!("ERROR: {}", msg));
        return Err(msg);
    }

    let mut secret_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut secret_bytes);
    let secret = STANDARD.encode(secret_bytes);

    emit(app, "[3/7] Creating Cloudflare tunnel...");
    let tunnel_id = cloudflare::create_tunnel(
        &client, &project.api_token, &project.account_id, &tunnel.tunnel_name, &secret,
    ).await.map_err(|e| { emit(app, &format!("ERROR: {}", e)); e })?;
    emit(app, &format!("  -> Tunnel ID: {}", tunnel_id));

    emit(app, "[4/7] Fetching tunnel token...");
    let token = cloudflare::get_tunnel_token(
        &client, &project.api_token, &project.account_id, &tunnel_id,
    ).await.map_err(|e| { emit(app, &format!("ERROR: {}", e)); e })?;
    emit(app, "  -> Token acquired");

    emit(app, "[5/7] Configuring ingress rules...");
    if let Err(e) = cloudflare::configure_ingress(
        &client, &project.api_token, &project.account_id, &tunnel_id,
        &tunnel.public_hostname, &tunnel.internal_service,
    ).await {
        emit(app, &format!("ERROR: {}", e));
        emit(app, "  -> Cleaning up orphaned tunnel...");
        let _ = cloudflare::delete_tunnel(&client, &project.api_token, &project.account_id, &tunnel_id).await;
        return Err(e);
    }
    emit(app, &format!("  -> {} -> {}", tunnel.public_hostname, tunnel.internal_service));

    emit(app, "[6/7] Creating DNS CNAME...");
    if let Err(e) = cloudflare::create_dns_cname(
        &client, &project.api_token, &zone_id, &tunnel.public_hostname, &tunnel_id,
    ).await {
        emit(app, &format!("ERROR: {}", e));
        emit(app, "  -> Cleaning up orphaned tunnel...");
        let _ = cloudflare::delete_tunnel(&client, &project.api_token, &project.account_id, &tunnel_id).await;
        return Err(e);
    }
    emit(app, &format!("  -> {} → {}.cfargotunnel.com", tunnel.public_hostname, tunnel_id));

    let local_pid: Option<u32> = if tunnel.target_type == "local" {
        emit(app, "[7/7] Starting local cloudflared process...");
        let a1 = app.clone();
        let a2 = app.clone();
        let token_arg = token.clone();

        let mut child = tokio::process::Command::new("cloudflared")
            .args(&["tunnel", "run", "--token", &token_arg])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                let msg = format!("Failed to spawn local cloudflared: {}", e);
                emit(app, &format!("ERROR: {}", msg));
                // Cleanup since deploy was otherwise successful
                msg
            })?;

        let pid = child.id();
        let stdout = BufReader::new(
            child.stdout.take().ok_or_else(|| "failed to capture cloudflared stdout".to_string())?,
        );
        let stderr = BufReader::new(
            child.stderr.take().ok_or_else(|| "failed to capture cloudflared stderr".to_string())?,
        );
        tokio::spawn(async move {
            let mut lines = stdout.lines();
            while let Ok(Some(l)) = lines.next_line().await { emit(&a1, &format!("[local] {}", l)); }
        });
        tokio::spawn(async move {
            let mut lines = stderr.lines();
            while let Ok(Some(l)) = lines.next_line().await { emit(&a2, &format!("[local] {}", l)); }
        });
        emit(app, "  -> Local cloudflared is now routing traffic");
        pid
    } else {
        emit(app, "[7/7] Deploying cloudflared via Helm...");
        run_silent("helm", &["repo", "add", "cloudflare", "https://cloudflare.github.io/helm-charts"])
            .await.ok();
        run_silent("helm", &["repo", "update"])
            .await.map_err(|e| {
                let msg = format!("helm repo update: {}", e);
                emit(app, &format!("ERROR: {}", msg));
                msg
            })?;

        if let Err(e) = run_streamed(app, "helm", &[
            "upgrade", "--install", "cloudflared", "cloudflare/cloudflare-tunnel",
            "--namespace", &tunnel.k8s_namespace, "--create-namespace",
            "--set", &format!("cloudflare.tunnelToken={}", token),
        ]).await {
            emit(app, &format!("ERROR: {}", e));
            emit(app, "  -> Cleaning up orphaned tunnel due to Helm failure...");
            let _ = cloudflare::delete_dns_cname(&client, &project.api_token, &zone_id, &tunnel.public_hostname).await;
            let _ = cloudflare::delete_tunnel(&client, &project.api_token, &project.account_id, &tunnel_id).await;
            return Err(e);
        }
        emit(app, "  -> Helm chart deployed");
        None
    };

    Ok((tunnel_id, local_pid))
}

// ── Browser-auth deploy — returns (tunnel_id, local_pid) ─────────────────────

pub async fn deploy_browser(
    app: &AppHandle,
    project_id: &str,
    tunnel: &TunnelConfig,
) -> Result<(String, Option<u32>), String> {
    let cert = project_cert_path(project_id);
    let cert_str = cert.to_str()
        .ok_or_else(|| "Cert path is not valid UTF-8".to_string())?;

    emit(app, "[1/6] Checking prerequisites...");
    let req_bins = if tunnel.target_type == "local" { vec!["cloudflared"] } else { vec!["cloudflared", "kubectl"] };
    for bin in req_bins {
        check_bin(bin).await.map_err(|e| { emit(app, &format!("ERROR: {}", e)); e })?;
    }
    if !cert.exists() {
        let msg = "Not authorized — authenticate this project first";
        emit(app, &format!("ERROR: {}", msg));
        return Err(msg.to_string());
    }
    emit(app, "  -> Prerequisites OK");

    if tunnel.target_type != "local" {
        emit(app, "[2/6] Checking Kubernetes connectivity...");
        if run_silent("kubectl", &["cluster-info"]).await.is_err() {
            let msg = "Could not connect to Kubernetes cluster. Make sure Docker Desktop, Minikube, orbstack, or another local cluster is running and your KUBECONFIG is correct.";
            emit(app, &format!("ERROR: {}", msg));
            return Err(msg.to_string());
        }
    } else {
        emit(app, "[2/6] Skipping K8s (local process mode)");
    }

    emit(app, "[3/6] Creating tunnel...");
    let out = run_silent("cloudflared", &["--origincert", cert_str, "tunnel", "create", &tunnel.tunnel_name])
        .await.map_err(|e| { emit(app, &format!("ERROR: {}", e)); e })?;
    let tunnel_id = out.lines()
        .find(|l| l.contains("with id "))
        .and_then(|l| l.split("with id ").nth(1))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "Could not parse tunnel ID from cloudflared output".to_string())?;
    emit(app, &format!("  -> Tunnel ID: {}", tunnel_id));

    emit(app, "[4/6] Routing DNS...");
    if let Err(e) = run_streamed(app, "cloudflared",
        &["--origincert", cert_str, "tunnel", "route", "dns", &tunnel.tunnel_name, &tunnel.public_hostname]).await
    {
        emit(app, &format!("ERROR: {}", e));
        emit(app, "  -> Cleaning up orphaned tunnel...");
        run_silent("cloudflared", &["--origincert", cert_str, "tunnel", "delete", "-f", &tunnel.tunnel_name]).await.ok();
        return Err(format!(
            "DNS routing failed for '{}'. Tunnel deleted. Make sure the hostname belongs to an authorized zone.",
            tunnel.public_hostname
        ));
    }

    let local_pid: Option<u32> = if tunnel.target_type == "local" {
        emit(app, "[5/6] Starting local cloudflared process...");
        let a1 = app.clone();
        let a2 = app.clone();

        let mut child = tokio::process::Command::new("cloudflared")
            .args(&["--origincert", cert_str, "tunnel", "run", "--url", &tunnel.internal_service, &tunnel.tunnel_name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                let msg = format!("Failed to spawn local cloudflared: {}", e);
                emit(app, &format!("ERROR: {}", msg));
                msg
            })?;

        let pid = child.id();
        let stdout = BufReader::new(
            child.stdout.take().ok_or_else(|| "failed to capture cloudflared stdout".to_string())?,
        );
        let stderr = BufReader::new(
            child.stderr.take().ok_or_else(|| "failed to capture cloudflared stderr".to_string())?,
        );
        tokio::spawn(async move {
            let mut lines = stdout.lines();
            while let Ok(Some(l)) = lines.next_line().await { emit(&a1, &format!("[local] {}", l)); }
        });
        tokio::spawn(async move {
            let mut lines = stderr.lines();
            while let Ok(Some(l)) = lines.next_line().await { emit(&a2, &format!("[local] {}", l)); }
        });
        emit(app, "  -> Local cloudflared is now routing traffic");
        pid
    } else {
        emit(app, "[5/6] Generating K8s manifests...");
        let creds_path = cf_home().join(format!("{}.json", tunnel_id));
        let creds_b64 = STANDARD.encode(
            std::fs::read(&creds_path)
                .map_err(|e| format!("Cannot read credentials {}: {}", creds_path.display(), e))?,
        );
        let manifest = build_manifest(
            &tunnel.k8s_namespace, &tunnel_id,
            &tunnel.public_hostname, &tunnel.internal_service, &creds_b64,
        );
        let manifest_path = std::env::temp_dir().join("cloudflared-manifest.yaml");
        std::fs::write(&manifest_path, &manifest).map_err(|e| format!("write manifest: {}", e))?;
        emit(app, &format!("  -> Manifest written to {}", manifest_path.display()));

        let manifest_str = manifest_path.to_str()
            .ok_or_else(|| "Manifest path is not valid UTF-8".to_string())?;

        emit(app, "[6/6] Applying K8s manifests...");
        if let Err(e) = run_streamed(app, "kubectl", &["apply", "-f", manifest_str]).await {
            emit(app, &format!("ERROR: {}", e));
            emit(app, "  -> Cleaning up orphaned tunnel due to kubectl error...");
            run_silent("cloudflared", &["--origincert", cert_str, "tunnel", "delete", "-f", &tunnel.tunnel_name]).await.ok();
            return Err(e);
        }
        None
    };

    Ok((tunnel_id, local_pid))
}

fn build_manifest(ns: &str, tunnel_id: &str, domain: &str, service: &str, creds_b64: &str) -> String {
    format!(
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
        ns = ns, tunnel_id = tunnel_id, domain = domain, service = service, creds_b64 = creds_b64,
    )
}

// ── Token-auth teardown ───────────────────────────────────────────────────────

pub async fn teardown_token(
    app: &AppHandle,
    project: &Project,
    tunnel_name: &str,
    hostname: &str,
    namespace: &str,
    target_type: &str,
    pid: Option<u32>,
) -> Result<(), String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    emit(app, "[1/4] Finding tunnel...");
    let tunnel_id = cloudflare::find_tunnel_id(
        &client, &project.api_token, &project.account_id, tunnel_name,
    ).await?
    .ok_or_else(|| {
        let msg = format!("No active tunnel named '{}'", tunnel_name);
        emit(app, &format!("ERROR: {}", msg));
        msg
    })?;
    emit(app, &format!("  -> Tunnel ID: {}", tunnel_id));

    emit(app, "[2/4] Stopping proxy daemon or K8s deployment...");
    if target_type == "k8s" {
        if run_streamed(app, "helm", &["uninstall", "cloudflared", "--namespace", namespace]).await.is_ok() {
            emit(app, "  -> Helm release removed");
        } else if run_streamed(app, "kubectl", &["delete", "namespace", namespace, "--ignore-not-found"]).await.is_ok() {
            emit(app, "  -> Namespace deleted");
        } else {
            emit(app, "  -> K8s cleanup skipped (resources not found)");
        }
    } else {
        if let Some(p) = pid {
            run_silent("kill", &["-TERM", &p.to_string()]).await.ok();
            emit(app, &format!("  -> cloudflared process {} terminated", p));
        } else {
            run_silent("pkill", &["-f", "cloudflared tunnel run"]).await.ok();
            emit(app, "  -> Local cloudflared processes killed");
        }
    }

    emit(app, "[3/4] Deleting DNS record...");
    let zone = cloudflare::extract_zone(hostname);
    if let Ok(zone_id) = cloudflare::get_zone_id(&client, &project.api_token, &project.account_id, &zone).await {
        match cloudflare::delete_dns_cname(&client, &project.api_token, &zone_id, hostname).await {
            Ok(()) => emit(app, &format!("  -> DNS record for '{}' deleted", hostname)),
            Err(e) => emit(app, &format!("  -> DNS warning: {} (continuing)", e)),
        }
    }

    emit(app, "[4/4] Deleting Cloudflare tunnel...");
    cloudflare::delete_tunnel(&client, &project.api_token, &project.account_id, &tunnel_id)
        .await.map_err(|e| { emit(app, &format!("ERROR: {}", e)); e })?;
    emit(app, "  -> Tunnel deleted");

    Ok(())
}

// ── Browser-auth teardown ─────────────────────────────────────────────────────

pub async fn teardown_browser(
    app: &AppHandle,
    project_id: &str,
    tunnel_name: &str,
    namespace: &str,
    target_type: &str,
    pid: Option<u32>,
) -> Result<(), String> {
    let cert = project_cert_path(project_id);
    let cert_str = cert.to_str()
        .ok_or_else(|| "Cert path is not valid UTF-8".to_string())?;

    emit(app, "[1/2] Stopping local process or K8s deployment...");
    if target_type == "k8s" {
        run_streamed(app, "kubectl", &["delete", "namespace", namespace, "--ignore-not-found"])
            .await.ok();
        emit(app, "  -> Namespace removed");
    } else {
        if let Some(p) = pid {
            run_silent("kill", &["-TERM", &p.to_string()]).await.ok();
            emit(app, &format!("  -> cloudflared process {} terminated", p));
        } else {
            run_silent("pkill", &["-f", "cloudflared tunnel run"]).await.ok();
            emit(app, "  -> Local cloudflared processes killed");
        }
    }

    emit(app, "[2/2] Deleting cloudflared tunnel...");
    run_streamed(app, "cloudflared", &["--origincert", cert_str, "tunnel", "delete", "-f", tunnel_name])
        .await.map_err(|e| { emit(app, &format!("ERROR: {}", e)); e })?;
    emit(app, "  -> Tunnel deleted");

    Ok(())
}
