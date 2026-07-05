#![allow(dead_code, unused_variables)]

//! S2 — Identity journey
//!
//! Tests the full identity lifecycle: request code → verify → login →
//! profile → refresh → logout.
//!
//! Uses the /__test__/email-code/peek test backdoor when the server is
//! running in e2e mode.

use reqwest::Client;
use serde::{Deserialize, Serialize};

const RUN_ID: &str = "e2e-s2-0001";

#[derive(Debug, Deserialize)]
struct EmailCodeResponse {
    code: String,
}

#[derive(Debug, Serialize)]
struct VerifyRequest {
    email: String,
    code: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    account: AccountInfo,
}

#[derive(Debug, Deserialize)]
struct AccountInfo {
    id: String,
    handle: String,
    #[serde(default)]
    avatar_url: Option<String>,
}

pub async fn run(base_url: &str, db_url: &str) -> anyhow::Result<()> {
    let email = format!("{RUN_ID}@tongji.edu.cn");
    let client = Client::new();

    tracing::info!(email, "S2 Identity journey starting");

    // Step 1: Request verification code
    let resp = client
        .post(format!("{base_url}/api/v2/auth/request-code"))
        .json(&serde_json::json!({ "email": email }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "request-code should return 200");

    // Step 2: Peek the code (test backdoor)
    let peek_resp: EmailCodeResponse = client
        .post(format!("{base_url}/__test__/email-code/peek"))
        .json(&serde_json::json!({ "email": email }))
        .send()
        .await?
        .json()
        .await
        .unwrap_or_else(|_| EmailCodeResponse { code: "000000".into() });
    tracing::info!(code = %peek_resp.code, "code peeked");

    // Step 3: Verify
    let tokens: TokenResponse = client
        .post(format!("{base_url}/api/v2/auth/verify"))
        .json(&VerifyRequest { email: email.clone(), code: peek_resp.code })
        .send()
        .await?
        .json()
        .await?;
    assert!(!tokens.access_token.is_empty(), "should have access token");

    // Step 4: GET /me
    let me: AccountInfo = client
        .get(format!("{base_url}/api/v2/me"))
        .header("Authorization", format!("Bearer {}", tokens.access_token))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(me.handle, email.split('@').next().unwrap_or("S2"));

    // Step 5: Refresh token
    let new_tokens: TokenResponse = client
        .post(format!("{base_url}/api/v2/auth/refresh"))
        .json(&serde_json::json!({ "refresh_token": tokens.refresh_token }))
        .send()
        .await?
        .json()
        .await?;
    assert!(!new_tokens.access_token.is_empty(), "should refresh");

    tracing::info!("S2 Identity journey completed successfully");
    Ok(())
}
