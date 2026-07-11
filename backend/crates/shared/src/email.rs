//! SMTP email sending for verification codes and notifications.
//!
//! Uses `lettre` for SMTP transport. When SMTP is not configured, logs
//! the email instead of sending.

use crate::config::Config;
use lettre::AsyncTransport;

/// Send an email via SMTP. Logs the email when SMTP is not configured.
///
/// Returns immediately (with a warning log) on any send failure — the caller
/// should not treat email as a hard dependency of the request path.
#[allow(dead_code)]
pub async fn send_email(config: &Config, to: &str, subject: &str, body: &str) {
    if config.smtp_host.is_empty() {
        tracing::info!(
            subject,
            "SMTP not configured — logging email instead (recipient and body redacted)"
        );
        return;
    }

    let from: lettre::message::Mailbox = match config.smtp_from.parse() {
        Ok(a) => a,
        Err(_) => {
            tracing::warn!("invalid SMTP_FROM address, falling back to default");
            match "noreply@yourtj.de".parse() {
                Ok(a) => a,
                Err(_) => {
                    tracing::error!("hardcoded fallback address failed to parse — this is a bug");
                    return;
                }
            }
        }
    };

    let to_addr: lettre::message::Mailbox = match to.parse() {
        Ok(a) => a,
        Err(_) => {
            tracing::warn!(subject, "invalid recipient email address");
            return;
        }
    };

    let email = match lettre::Message::builder()
        .from(from)
        .to(to_addr)
        .subject(subject)
        .body(body.to_string())
    {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, subject, "failed to build email message");
            return;
        }
    };

    let creds = lettre::transport::smtp::authentication::Credentials::new(
        config.smtp_username.clone(),
        config.smtp_password.clone(),
    );

    let transport = match lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(
        &config.smtp_host,
    )
    .map(|t| t.credentials(creds).port(config.smtp_port).build())
    {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, host = %config.smtp_host, "failed to create SMTP transport");
            return;
        }
    };

    match transport.send(email).await {
        Ok(_) => tracing::info!(subject, "email sent"),
        Err(e) => tracing::warn!(error = %e, subject, "failed to send email"),
    }
}
