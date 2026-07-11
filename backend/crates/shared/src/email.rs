//! Outbound email delivery through Cloudflare Email Sending or legacy SMTP.

use lettre::message::{Mailbox, MultiPart};
use lettre::AsyncTransport;
use serde::{Deserialize, Serialize};

use crate::config::{Config, EmailProvider};
use crate::{AppError, AppResult};

const EMAIL_HTTP_TIMEOUT_SECONDS: u64 = 10;

#[derive(Serialize)]
struct CloudflareEmailRequest<'a> {
    to: &'a str,
    from: &'a str,
    subject: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct CloudflareEmailEnvelope {
    success: bool,
    #[serde(default)]
    errors: Vec<CloudflareApiError>,
    result: Option<CloudflareEmailResult>,
}

#[derive(Debug, Deserialize)]
struct CloudflareApiError {
    code: i64,
}

#[derive(Debug, Deserialize)]
struct CloudflareEmailResult {
    message_id: String,
    #[serde(default)]
    delivered: Vec<String>,
    #[serde(default)]
    queued: Vec<String>,
    #[serde(default)]
    permanent_bounces: Vec<String>,
}

/// Deliver one email without logging its recipient or body.
///
/// `EMAIL_PROVIDER=log` is a redacted local-development sink. Configured Cloudflare and SMTP
/// providers fail closed with `SERVICE_UNAVAILABLE` when the upstream does not accept delivery.
pub async fn send_email(
    config: &Config,
    to: &str,
    subject: &str,
    text: &str,
    html: Option<&str>,
) -> AppResult<()> {
    if to.parse::<Mailbox>().is_err() {
        tracing::warn!(subject, "outbound email rejected an invalid recipient mailbox");
        return Err(AppError::BadRequest("invalid recipient email address".into()));
    }

    match config.email_provider {
        EmailProvider::Log => {
            tracing::warn!(subject, "email log provider accepted redacted local delivery");
            Ok(())
        }
        EmailProvider::Cloudflare => send_cloudflare(config, to, subject, text, html).await,
        EmailProvider::Smtp => send_smtp(config, to, subject, text, html).await,
    }
}

async fn send_cloudflare(
    config: &Config,
    to: &str,
    subject: &str,
    text: &str,
    html: Option<&str>,
) -> AppResult<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(EMAIL_HTTP_TIMEOUT_SECONDS))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| {
            tracing::warn!(?error, "failed to build Cloudflare email client");
            AppError::ServiceUnavailable
        })?;
    let endpoint = format!(
        "{}/accounts/{}/email/sending/send",
        config.cloudflare_email_api_base_url.trim_end_matches('/'),
        config.cloudflare_email_account_id
    );
    let payload = CloudflareEmailRequest { to, from: &config.email_from, subject, text, html };
    let response = client
        .post(endpoint)
        .bearer_auth(&config.cloudflare_email_api_token)
        .json(&payload)
        .send()
        .await
        .map_err(|error| {
            tracing::warn!(?error, "Cloudflare email request failed");
            AppError::ServiceUnavailable
        })?;
    let status = response.status();
    if !status.is_success() {
        tracing::warn!(%status, "Cloudflare email request was rejected");
        return Err(AppError::ServiceUnavailable);
    }
    let envelope = response.json::<CloudflareEmailEnvelope>().await.map_err(|error| {
        tracing::warn!(?error, "Cloudflare email returned an invalid response");
        AppError::ServiceUnavailable
    })?;
    let delivery = accepted_cloudflare_delivery(&envelope).ok_or_else(|| {
        let error_codes: Vec<i64> = envelope.errors.iter().map(|error| error.code).collect();
        tracing::warn!(?error_codes, "Cloudflare did not accept email delivery");
        AppError::ServiceUnavailable
    })?;
    tracing::info!(
        message_id = %delivery.message_id,
        delivered = delivery.delivered.len(),
        queued = delivery.queued.len(),
        "Cloudflare email accepted"
    );
    Ok(())
}

fn accepted_cloudflare_delivery(
    envelope: &CloudflareEmailEnvelope,
) -> Option<&CloudflareEmailResult> {
    let result = envelope.result.as_ref()?;
    (envelope.success
        && envelope.errors.is_empty()
        && !result.message_id.trim().is_empty()
        && result.permanent_bounces.is_empty())
    .then_some(result)
}

async fn send_smtp(
    config: &Config,
    to: &str,
    subject: &str,
    text: &str,
    html: Option<&str>,
) -> AppResult<()> {
    let from = config.email_from.parse::<Mailbox>().map_err(|error| {
        tracing::warn!(?error, "configured outbound sender mailbox is invalid");
        AppError::ServiceUnavailable
    })?;
    let recipient = to.parse::<Mailbox>().map_err(|error| {
        tracing::warn!(?error, subject, "recipient mailbox parsing failed");
        AppError::BadRequest("invalid recipient email address".into())
    })?;
    let builder = lettre::Message::builder().from(from).to(recipient).subject(subject);
    let message = match html {
        Some(html_body) => builder
            .multipart(MultiPart::alternative_plain_html(text.to_string(), html_body.to_string())),
        None => builder.body(text.to_string()),
    }
    .map_err(|error| {
        tracing::warn!(?error, subject, "failed to build outbound email");
        AppError::ServiceUnavailable
    })?;
    let mut transport_builder = lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(
        &config.smtp_host,
    )
    .map_err(|error| {
        tracing::warn!(?error, "failed to configure SMTP transport");
        AppError::ServiceUnavailable
    })?;
    if !config.smtp_username.is_empty() {
        let credentials = lettre::transport::smtp::authentication::Credentials::new(
            config.smtp_username.clone(),
            config.smtp_password.clone(),
        );
        transport_builder = transport_builder.credentials(credentials);
    }
    let transport = transport_builder.port(config.smtp_port).build();
    transport.send(message).await.map_err(|error| {
        tracing::warn!(?error, "SMTP email request failed");
        AppError::ServiceUnavailable
    })?;
    tracing::info!(subject, "SMTP email accepted");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{accepted_cloudflare_delivery, CloudflareEmailEnvelope};

    #[test]
    fn accepts_successful_cloudflare_result_with_message_id_and_without_bounces() {
        for result in [
            r#"{"success":true,"errors":[],"result":{"message_id":"one","delivered":["recipient@example.com"],"queued":[],"permanent_bounces":[]}}"#,
            r#"{"success":true,"errors":[],"result":{"message_id":"two","delivered":[],"queued":["recipient@example.com"],"permanent_bounces":[]}}"#,
            r#"{"success":true,"errors":[],"result":{"message_id":"three","delivered":[],"queued":[],"permanent_bounces":[]}}"#,
        ] {
            let envelope: CloudflareEmailEnvelope =
                serde_json::from_str(result).expect("valid Cloudflare fixture");
            assert!(accepted_cloudflare_delivery(&envelope).is_some());
        }
    }

    #[test]
    fn rejects_unsuccessful_or_bounced_cloudflare_result() {
        for result in [
            r#"{"success":false,"errors":[{"code":1000}],"result":null}"#,
            r#"{"success":true,"errors":[],"result":{"message_id":"","delivered":[],"queued":[],"permanent_bounces":[]}}"#,
            r#"{"success":true,"errors":[],"result":{"message_id":"four","delivered":[],"queued":[],"permanent_bounces":["recipient@example.com"]}}"#,
        ] {
            let envelope: CloudflareEmailEnvelope =
                serde_json::from_str(result).expect("valid Cloudflare fixture");
            assert!(accepted_cloudflare_delivery(&envelope).is_none());
        }
    }
}
