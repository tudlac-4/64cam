use std::time::{Duration, Instant};

use chrono::Utc;
use common::ws::{EventPayload, NodeCameraConfig};
use tokio::sync::mpsc;
use uuid::Uuid;

const POLL_INTERVAL:    Duration = Duration::from_secs(3);
const RENEW_INTERVAL:   Duration = Duration::from_secs(480); // 8 min < 10 min expiry
const COOLDOWN:         Duration = Duration::from_secs(10);

pub async fn run(cam: NodeCameraConfig, tx: mpsc::UnboundedSender<EventPayload>) {
    let url      = cam.onvif_url.as_deref().unwrap_or("");
    let user     = cam.onvif_username.as_deref().unwrap_or("");
    let password = cam.onvif_password.as_deref().unwrap_or("");

    loop {
        if let Err(e) = subscribe_and_poll(cam.id, url, user, password, &tx).await {
            tracing::warn!("ONVIF [{}] error: {e} — retrying in 15s", cam.name);
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

async fn subscribe_and_poll(
    camera_id: Uuid,
    event_url: &str,
    user:      &str,
    password:  &str,
    tx:        &mpsc::UnboundedSender<EventPayload>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()?;

    // ── CreatePullPointSubscription ──────────────────────────────────────────
    let create_body = soap_envelope(user, password, &format!(
        r#"<tev:CreatePullPointSubscription xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
             <tev:InitialTerminationTime>PT600S</tev:InitialTerminationTime>
           </tev:CreatePullPointSubscription>"#
    ));

    let resp = client
        .post(event_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(create_body)
        .send()
        .await?
        .text()
        .await?;

    let sub_addr = extract_tag(&resp, "Address")
        .ok_or_else(|| anyhow::anyhow!("no subscription address in CreatePullPointSubscription response"))?;

    tracing::debug!("ONVIF [camera {camera_id}] subscribed → {sub_addr}");

    let mut last_motion = Instant::now() - COOLDOWN * 2;
    let mut last_renew  = Instant::now();

    loop {
        // ── Renew subscription before it expires ────────────────────────────
        if last_renew.elapsed() >= RENEW_INTERVAL {
            let renew_body = soap_envelope(user, password, &format!(
                r#"<wsnt:Renew xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2">
                     <wsnt:TerminationTime>PT600S</wsnt:TerminationTime>
                   </wsnt:Renew>"#
            ));
            if client
                .post(sub_addr)
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .body(renew_body)
                .send()
                .await
                .is_ok()
            {
                last_renew = Instant::now();
            }
        }

        // ── PullMessages ────────────────────────────────────────────────────
        let pull_body = soap_envelope(user, password,
            r#"<tev:PullMessages xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
                 <tev:Timeout>PT3S</tev:Timeout>
                 <tev:MessageLimit>10</tev:MessageLimit>
               </tev:PullMessages>"#,
        );

        match client
            .post(&sub_addr)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(pull_body)
            .send()
            .await
        {
            Ok(r) => {
                let text = r.text().await.unwrap_or_default();
                if has_motion(&text) && last_motion.elapsed() >= COOLDOWN {
                    last_motion = Instant::now();
                    let _ = tx.send(EventPayload {
                        camera_id,
                        occurred_at: Utc::now(),
                        source:      "onvif".into(),
                        score:       None,
                    });
                }
            }
            Err(e) => {
                // Subscription may have expired; bubble up to trigger recreation
                return Err(e.into());
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Scan response XML for an ONVIF motion = true signal.
/// ONVIF XML namespace prefixes vary by manufacturer, so we scan for the
/// `IsMotion` attribute name and a nearby `true` (or `1`) value.
fn has_motion(xml: &str) -> bool {
    if let Some(pos) = xml.find("IsMotion") {
        // Scan the next 80 chars for a "true" or "1" value
        let end = (pos + 80).min(xml.len());
        let window = &xml[pos..end];
        window.contains("true") || window.contains(">1<")
    } else {
        false
    }
}

/// Build a minimal SOAP 1.2 envelope. Includes WS-Security UsernameToken when
/// credentials are provided (required by most ONVIF cameras by default).
fn soap_envelope(user: &str, password: &str, body_content: &str) -> String {
    let security = if !user.is_empty() {
        format!(
            r#"<s:Header>
               <wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
                 <wsse:UsernameToken>
                   <wsse:Username>{user}</wsse:Username>
                   <wsse:Password>{password}</wsse:Password>
                 </wsse:UsernameToken>
               </wsse:Security>
             </s:Header>"#
        )
    } else {
        "<s:Header/>".into()
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  {security}
  <s:Body>{body_content}</s:Body>
</s:Envelope>"#
    )
}

/// Extract the text content of the first occurrence of `<tag>…</tag>`.
fn extract_tag<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open  = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end   = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim())
}
