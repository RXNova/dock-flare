use reqwest::Client;
use serde::{Deserialize, Serialize};

// ── Response envelope ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CfResp<T> {
    pub result: Option<T>,
    pub success: bool,
    pub errors: Vec<serde_json::Value>,
}

impl<T> CfResp<T> {
    pub fn into_result(self) -> Result<T, String> {
        if self.success {
            self.result.ok_or_else(|| "empty result".to_string())
        } else {
            Err(format!("Cloudflare API error: {:?}", self.errors))
        }
    }
}

// ── Tunnel types ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CfTunnel {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct CfZone {
    pub id: String,
}

#[derive(Deserialize)]
pub struct CfDnsRecord {
    pub id: String,
}

// ── Tunnels ───────────────────────────────────────────────────────────────────

pub async fn list_tunnels(
    client: &Client,
    api_token: &str,
    account_id: &str,
) -> Result<Vec<CfTunnel>, String> {
    let body: CfResp<Vec<CfTunnel>> = client
        .get(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel",
            account_id
        ))
        .bearer_auth(api_token)
        .query(&[("is_deleted", "false")])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(body.result.unwrap_or_default())
}

pub async fn find_tunnel_id(
    client: &Client,
    api_token: &str,
    account_id: &str,
    name: &str,
) -> Result<Option<String>, String> {
    let body: CfResp<Vec<CfTunnel>> = client
        .get(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel",
            account_id
        ))
        .bearer_auth(api_token)
        .query(&[("name", name), ("is_deleted", "false")])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(body.result.unwrap_or_default().into_iter().next().map(|t| t.id))
}

pub async fn create_tunnel(
    client: &Client,
    api_token: &str,
    account_id: &str,
    name: &str,
    secret: &str,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Req<'a> { name: &'a str, tunnel_secret: &'a str }
    #[derive(Deserialize)]
    struct Res { id: String }

    client
        .post(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel",
            account_id
        ))
        .bearer_auth(api_token)
        .json(&Req { name, tunnel_secret: secret })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<CfResp<Res>>()
        .await
        .map_err(|e| e.to_string())?
        .into_result()
        .map(|r| r.id)
}

pub async fn get_tunnel_token(
    client: &Client,
    api_token: &str,
    account_id: &str,
    tunnel_id: &str,
) -> Result<String, String> {
    client
        .get(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel/{}/token",
            account_id, tunnel_id
        ))
        .bearer_auth(api_token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<CfResp<String>>()
        .await
        .map_err(|e| e.to_string())?
        .into_result()
}

pub async fn configure_ingress(
    client: &Client,
    api_token: &str,
    account_id: &str,
    tunnel_id: &str,
    hostname: &str,
    service: &str,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Rule {
        #[serde(skip_serializing_if = "Option::is_none")]
        hostname: Option<String>,
        service: String,
    }
    #[derive(Serialize)]
    struct Cfg { ingress: Vec<Rule> }
    #[derive(Serialize)]
    struct Body { config: Cfg }

    client
        .put(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel/{}/configurations",
            account_id, tunnel_id
        ))
        .bearer_auth(api_token)
        .json(&Body {
            config: Cfg {
                ingress: vec![
                    Rule { hostname: Some(hostname.to_string()), service: service.to_string() },
                    Rule { hostname: None, service: "http_status:404".to_string() },
                ],
            },
        })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<CfResp<serde_json::Value>>()
        .await
        .map_err(|e| e.to_string())?
        .into_result()
        .map(|_| ())
}

pub async fn delete_tunnel(
    client: &Client,
    api_token: &str,
    account_id: &str,
    tunnel_id: &str,
) -> Result<(), String> {
    // Clean up active connections first (best-effort)
    let _ = client
        .delete(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel/{}/connections",
            account_id, tunnel_id
        ))
        .bearer_auth(api_token)
        .send()
        .await;

    client
        .delete(format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfdtunnel/{}",
            account_id, tunnel_id
        ))
        .bearer_auth(api_token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<CfResp<serde_json::Value>>()
        .await
        .map_err(|e| e.to_string())?
        .into_result()
        .map(|_| ())
}

// ── DNS ───────────────────────────────────────────────────────────────────────

pub fn extract_zone(domain: &str) -> String {
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 2..].join(".")
    } else {
        domain.to_string()
    }
}

pub async fn get_zone_id(
    client: &Client,
    api_token: &str,
    account_id: &str,
    zone: &str,
) -> Result<String, String> {
    let _ = account_id; // not used by the zones endpoint but kept for API consistency
    let body: CfResp<Vec<CfZone>> = client
        .get("https://api.cloudflare.com/client/v4/zones")
        .bearer_auth(api_token)
        .query(&[("name", zone)])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    body.result
        .unwrap_or_default()
        .into_iter()
        .next()
        .map(|z| z.id)
        .ok_or_else(|| format!("No zone found for '{}'", zone))
}

pub async fn create_dns_cname(
    client: &Client,
    api_token: &str,
    zone_id: &str,
    hostname: &str,
    tunnel_id: &str,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Rec {
        r#type: &'static str,
        name: String,
        content: String,
        ttl: u32,
        proxied: bool,
    }
    client
        .post(format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            zone_id
        ))
        .bearer_auth(api_token)
        .json(&Rec {
            r#type: "CNAME",
            name: hostname.to_string(),
            content: format!("{}.cfargotunnel.com", tunnel_id),
            ttl: 1,
            proxied: true,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<CfResp<serde_json::Value>>()
        .await
        .map_err(|e| e.to_string())?
        .into_result()
        .map(|_| ())
}

pub async fn delete_dns_cname(
    client: &Client,
    api_token: &str,
    zone_id: &str,
    hostname: &str,
) -> Result<(), String> {
    let body: CfResp<Vec<CfDnsRecord>> = client
        .get(format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            zone_id
        ))
        .bearer_auth(api_token)
        .query(&[("name", hostname), ("type", "CNAME")])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    for record in body.result.unwrap_or_default() {
        let _ = client
            .delete(format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                zone_id, record.id
            ))
            .bearer_auth(api_token)
            .send()
            .await;
    }
    Ok(())
}
