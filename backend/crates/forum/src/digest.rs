//! Weekly email digest for forum users.
//!
//! Queries opted-in users (`"email_digest": true` in their notification prefs),
//! builds a summary of top threads, unread notifications, and new badges from
//! the past week, then sends the digest via SMTP.
//!
//! Called by the bootstrap scheduled task (see `api/src/bootstrap.rs`). This
//! module gracefully degrades to logging when SMTP is not configured.

use chrono::{DateTime, Utc};
use shared::config::Config;
use sqlx::PgPool;

const FORUM_BASE_URL: &str = "https://yourtj.de";
const THREAD_URL_PREFIX: &str = "/forum/threads/";

// ---------------------------------------------------------------------------
// Query row types
// ---------------------------------------------------------------------------

/// A user who has opted into the email digest.
#[derive(Debug, sqlx::FromRow)]
struct DigestSubscriber {
    account_id: i64,
    email: String,
    handle: String,
}

/// A top thread from the past week, joined with the author handle.
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct TopThread {
    id: i64,
    title: String,
    reply_count: i32,
    vote_count: i32,
    hot_score: Option<f64>,
    last_activity_at: DateTime<Utc>,
    author_handle: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct WeeklyBadge {
    slug: String,
    name: String,
    awarded_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the weekly email digest.
///
/// Fetches all subscribers, gathers per-user data, builds HTML, and sends the
/// email. Maintains a 1-second delay between recipients to avoid appearing as
/// a bulk sender. Gracefully skips individual users on query errors and logs
/// the email body when SMTP is not configured.
///
/// This function is safe to call on every tick — it is a no-op when there are
/// no subscribers or when all queries fail.
#[allow(dead_code)]
pub async fn run_digest(pool: &PgPool, config: &Config) {
    let subscribers = match get_subscribers(pool).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "failed to query digest subscribers");
            return;
        }
    };

    if subscribers.is_empty() {
        tracing::info!("no email digest subscribers — skipping");
        return;
    }

    for sub in &subscribers {
        let top_threads = match get_top_threads(pool).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(error = %e, account_id = sub.account_id, "failed to query top threads");
                continue;
            }
        };

        let unread_count = match get_unread_count(pool, sub.account_id).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, account_id = sub.account_id, "failed to query unread count");
                continue;
            }
        };

        let badges = match get_new_badges(pool, sub.account_id).await {
            Ok(b) => b,
            Err(e) => {
                tracing::error!(error = %e, account_id = sub.account_id, "failed to query new badges");
                continue;
            }
        };

        let html = build_html(&sub.handle, &top_threads, unread_count, &badges);
        shared::email::send_email(config, &sub.email, "YourTJ 论坛周报", &html).await;

        // Rate-limit: 1-second gap between recipients
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    tracing::info!(count = subscribers.len(), "email digest complete");
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// All accounts with `email_digest: true` in their notification prefs.
async fn get_subscribers(pool: &PgPool) -> Result<Vec<DigestSubscriber>, sqlx::Error> {
    sqlx::query_as::<_, DigestSubscriber>(
        r#"
        SELECT np.account_id, a.email, a.handle
        FROM forum.notification_prefs np
        JOIN identity.accounts a ON a.id = np.account_id
        WHERE np.prefs->>'email_digest' = 'true'
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Top 5 active threads from the past week, ordered by hot score.
async fn get_top_threads(pool: &PgPool) -> Result<Vec<TopThread>, sqlx::Error> {
    sqlx::query_as::<_, TopThread>(
        r#"
        SELECT t.id, t.title, t.reply_count, t.vote_count, t.hot_score,
               t.last_activity_at, a.handle AS author_handle
        FROM forum.threads t
        JOIN identity.accounts a ON a.id = t.author_id
        WHERE t.deleted_at IS NULL
          AND t.hidden_at IS NULL
          AND t.last_activity_at > now() - interval '7 days'
        ORDER BY t.hot_score DESC NULLS LAST
        LIMIT 5
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Number of unread notifications for a given account.
async fn get_unread_count(pool: &PgPool, account_id: i64) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM forum.notifications
        WHERE account_id = $1 AND read_at IS NULL
        "#,
    )
    .bind(account_id)
    .fetch_one(pool)
    .await
}

/// Badges awarded to this account in the past week.
async fn get_new_badges(pool: &PgPool, account_id: i64) -> Result<Vec<WeeklyBadge>, sqlx::Error> {
    sqlx::query_as::<_, WeeklyBadge>(
        r#"
        SELECT b.slug, b.name, ab.awarded_at
        FROM platform.account_badges ab
        JOIN platform.badges b ON b.id = ab.badge_id
        WHERE ab.account_id = $1
          AND ab.awarded_at > now() - interval '7 days'
        "#,
    )
    .bind(account_id)
    .fetch_all(pool)
    .await
}

// ---------------------------------------------------------------------------
// HTML template
// ---------------------------------------------------------------------------

/// Build the digest HTML email body.
fn build_html(
    handle: &str,
    threads: &[TopThread],
    unread_count: i64,
    badges: &[WeeklyBadge],
) -> String {
    let thread_rows: String = threads
        .iter()
        .map(|t| {
            let url = format!("{}{}{}", FORUM_BASE_URL, THREAD_URL_PREFIX, t.id);
            let title = html_escape(&t.title);
            let author = html_escape(&t.author_handle);
            format!(
                r#"  <div style="margin-bottom:12px;padding:8px;border-left:3px solid #4A90D9;">
    <a href="{}" style="font-size:16px;color:#4A90D9;">{}</a>
    <p style="color:#666;font-size:12px;">
      作者: {} · {} 回复 · {} 赞
    </p>
  </div>"#,
                url, title, author, t.reply_count, t.vote_count,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let badge_list: String = if badges.is_empty() {
        String::new()
    } else {
        let items: String = badges
            .iter()
            .map(|b| {
                let name = html_escape(&b.name);
                let slug = html_escape(&b.slug);
                format!("    <li>{} ({})</li>", name, slug)
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"
  <h2>新获得的徽章</h2>
  <ul>
{}
  </ul>"#,
            items
        )
    };

    let handle = html_escape(handle);

    format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="font-family:sans-serif;max-width:600px;margin:0 auto;">
  <h1>YourTJ 论坛周报</h1>
  <p>你好 {handle}，这是过去一周的论坛动态摘要。</p>

  <h2>热点主题</h2>
{thread_rows}

  <h2>未读通知</h2>
  <p>你有 {unread_count} 条未读通知。</p>
{badge_list}
  <hr>
  <p style="color:#999;font-size:12px;">
    你可以随时在 设置 → 通知偏好 中取消订阅此周报。
  </p>
</body>
</html>"#,
        handle = handle,
        thread_rows = thread_rows,
        unread_count = unread_count,
        badge_list = badge_list,
    )
}

/// Minimal HTML escaping for user-controlled strings in the email.
fn html_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("hello"), "hello");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"'"), "&quot;&#39;");
    }

    #[test]
    fn test_build_html_empty() {
        let html = build_html("TestUser", &[], 0, &[]);
        assert!(html.contains("TestUser"));
        assert!(html.contains("0 条未读通知"));
        assert!(!html.contains("新获得的徽章"));
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_build_html_with_badges() {
        let badges = vec![WeeklyBadge {
            slug: "first-thread".into(),
            name: "首次发帖".into(),
            awarded_at: Utc::now(),
        }];
        let html = build_html("User", &[], 3, &badges);
        assert!(html.contains("首次发帖"));
        assert!(html.contains("first-thread"));
        assert!(html.contains("3 条未读通知"));
        assert!(html.contains("新获得的徽章"));
    }

    #[test]
    fn test_build_html_with_threads() {
        let threads = vec![TopThread {
            id: 42,
            title: "测试标题".into(),
            reply_count: 5,
            vote_count: 10,
            hot_score: Some(100.0),
            last_activity_at: Utc::now(),
            author_handle: "作者A".into(),
        }];
        let html = build_html("User", &threads, 1, &[]);
        assert!(html.contains("测试标题"));
        assert!(html.contains("forum/threads/42"));
        assert!(html.contains("作者A"));
        assert!(html.contains("5 回复"));
        assert!(html.contains("10 赞"));
    }

    #[test]
    fn test_build_html_escapes_user_content() {
        let threads = vec![TopThread {
            id: 1,
            title: "<b>恶意</b>".into(),
            reply_count: 0,
            vote_count: 0,
            hot_score: None,
            last_activity_at: Utc::now(),
            author_handle: "&黑客".into(),
        }];
        let html = build_html("<script>alert(1)</script>", &threads, 0, &[]);
        assert!(html.contains("&lt;b&gt;恶意&lt;/b&gt;"));
        assert!(html.contains("&amp;黑客"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    }
}
