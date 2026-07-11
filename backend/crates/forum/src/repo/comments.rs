use shared::AppResult;
use sqlx::{FromRow, PgConnection, PgPool};

use crate::models::CommentRowJoined;

use super::{base64_decode_str, base64_encode_str};

#[derive(Debug, FromRow)]
struct ThreadPostingState {
    author_id: Option<i64>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    hidden_at: Option<chrono::DateTime<chrono::Utc>>,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    archived_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Created comment plus recipient metadata read in the same transaction.
pub struct CommentCreateOutcome {
    pub row: CommentRowJoined,
    pub thread_author_id: Option<i64>,
    pub quoted_author_id: Option<i64>,
}

/// Canonical source stored for a comment mutation.
pub struct CommentSource<'a> {
    pub body: &'a str,
    pub content_format: &'a str,
    pub image_references: &'a [media::attachments::ForumAssetReference],
}

/// Canonical source and compare-and-swap facts for one comment edit.
pub struct CommentUpdateSource<'a> {
    pub body: &'a str,
    pub content_format: &'a str,
    pub expected_version: i64,
    pub is_queued: bool,
    pub image_references: &'a [media::attachments::ForumAssetReference],
}

/// Return whether the parent thread currently permits author comment edits.
pub async fn thread_allows_comment_edits(pool: &PgPool, thread_id: i64) -> AppResult<bool> {
    let allows_edits: Option<bool> = sqlx::query_scalar(
        "SELECT deleted_at IS NULL AND hidden_at IS NULL AND archived_at IS NULL \
         FROM forum.threads WHERE id = $1",
    )
    .bind(thread_id)
    .fetch_optional(pool)
    .await?;
    allows_edits.ok_or(shared::AppError::NotFound)
}

struct NewComment<'a> {
    thread_id: i64,
    parent_id: Option<i64>,
    path: &'a str,
    author_id: i64,
    body: &'a str,
    content_format: &'a str,
    quoted_comment_id: Option<i64>,
    is_hidden: bool,
}

/// List comments for a thread with cursor pagination.
/// Ordered by `path` ASC for correct nested (楼中楼) display.
/// When `current_user_id` is `Some`, comments by users the current user has
/// ignored are excluded.
pub async fn list_comments(
    pool: &PgPool,
    thread_id: i64,
    cursor: Option<&str>,
    limit: i64,
    current_user_id: Option<i64>,
    can_view_moderated_parent: bool,
) -> AppResult<(Vec<CommentRowJoined>, Option<String>)> {
    let thread_is_readable: bool = sqlx::query_scalar(
        "SELECT EXISTS( \
           SELECT 1 FROM forum.threads \
           WHERE id = $1 AND ($2 OR (deleted_at IS NULL AND hidden_at IS NULL)) \
         )",
    )
    .bind(thread_id)
    .bind(can_view_moderated_parent)
    .fetch_one(pool)
    .await?;
    if !thread_is_readable {
        return Err(shared::AppError::NotFound);
    }

    let cursor_path: Option<String> = cursor
        .map(base64_decode_str)
        .transpose()
        .map_err(|_| shared::AppError::BadRequest("invalid cursor".into()))?;

    let rows = if let Some(ref cp) = cursor_path {
        sqlx::query_as::<_, CommentRowJoined>(
            "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                    c.body, c.content_format, c.content_version, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                    c.quoted_comment_id, \
                    a.handle AS author_handle \
             FROM forum.comments c \
             JOIN identity.accounts a ON a.id = c.author_id \
             WHERE c.thread_id = $1 AND c.deleted_at IS NULL AND c.hidden_at IS NULL \
               AND c.path > $3 \
               AND ($4::bigint IS NULL OR NOT forum.user_pair_blocked($4, c.author_id)) \
             ORDER BY c.path ASC \
             LIMIT $2",
        )
        .bind(thread_id)
        .bind(limit + 1)
        .bind(cp)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, CommentRowJoined>(
            "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                    c.body, c.content_format, c.content_version, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                    c.quoted_comment_id, \
                    a.handle AS author_handle \
             FROM forum.comments c \
             JOIN identity.accounts a ON a.id = c.author_id \
             WHERE c.thread_id = $1 AND c.deleted_at IS NULL AND c.hidden_at IS NULL \
               AND ($3::bigint IS NULL OR NOT forum.user_pair_blocked($3, c.author_id)) \
             ORDER BY c.path ASC \
             LIMIT $2",
        )
        .bind(thread_id)
        .bind(limit + 1)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?
    };

    let has_more = rows.len() > limit as usize;
    let items = if has_more { rows[..limit as usize].to_vec() } else { rows };
    let next_cursor = if has_more {
        items.last().and_then(|r| r.path.as_ref()).map(|p| base64_encode_str(p))
    } else {
        None
    };

    Ok((items, next_cursor))
}

/// Create a comment with materialized path for 楼中楼 ordering.
///
/// If `parent_id` is provided, the path is computed as `{parent_path}.{next_sibling}`.
/// Otherwise the path is the next zero-padded top-level index.
/// Uses a transaction with row-level locks for race-free path generation.
pub async fn create_comment(
    pool: &PgPool,
    thread_id: i64,
    author_id: i64,
    source: CommentSource<'_>,
    parent_id: Option<i64>,
    quoted_comment_id: Option<i64>,
    is_hidden: bool,
) -> AppResult<CommentCreateOutcome> {
    let mut tx = pool.begin().await?;

    let thread_state: Option<ThreadPostingState> = sqlx::query_as(
        "SELECT author_id, deleted_at, hidden_at, closed_at, archived_at \
         FROM forum.threads WHERE id = $1 FOR UPDATE",
    )
    .bind(thread_id)
    .fetch_optional(&mut *tx)
    .await?;
    let thread_state = thread_state.ok_or(shared::AppError::NotFound)?;
    if thread_state.deleted_at.is_some() || thread_state.hidden_at.is_some() {
        return Err(shared::AppError::NotFound);
    }
    if thread_state.archived_at.is_some() {
        return Err(shared::AppError::Conflict("thread is archived".into()));
    }
    if thread_state.closed_at.is_some() {
        return Err(shared::AppError::Conflict("thread is closed".into()));
    }

    let quoted_author_id = if let Some(quoted_comment_id) = quoted_comment_id {
        let quoted_author: Option<(Option<i64>,)> = sqlx::query_as(
            "SELECT author_id FROM forum.comments \
             WHERE id = $1 AND thread_id = $2 \
               AND deleted_at IS NULL AND hidden_at IS NULL \
             FOR KEY SHARE",
        )
        .bind(quoted_comment_id)
        .bind(thread_id)
        .fetch_optional(&mut *tx)
        .await?;
        quoted_author
            .ok_or_else(|| {
                shared::AppError::BadRequest(
                    "quoted comment must be an available comment in this thread".into(),
                )
            })?
            .0
    } else {
        None
    };

    let parent_author_id = if let Some(parent_id) = parent_id {
        sqlx::query_scalar(
            "SELECT author_id FROM forum.comments \
             WHERE id = $1 AND thread_id = $2 \
               AND deleted_at IS NULL AND hidden_at IS NULL",
        )
        .bind(parent_id)
        .bind(thread_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten()
    } else {
        None
    };
    let mut direct_account_ids: Vec<i64> =
        [thread_state.author_id, quoted_author_id, parent_author_id]
            .into_iter()
            .flatten()
            .filter(|target_id| *target_id != author_id)
            .collect();
    direct_account_ids.sort_unstable();
    direct_account_ids.dedup();
    for target_id in direct_account_ids {
        super::relationships::lock_pair_unblocked(&mut tx, author_id, target_id).await?;
    }

    let row = if let Some(pid) = parent_id {
        // Lock the parent comment row to prevent concurrent sibling inserts.
        // Fetch parent path inside the same transaction with FOR UPDATE.
        let parent_path: Option<String> = sqlx::query_scalar(
            "SELECT path FROM forum.comments \
             WHERE id = $1 AND thread_id = $2 \
               AND deleted_at IS NULL AND hidden_at IS NULL \
             FOR UPDATE",
        )
        .bind(pid)
        .bind(thread_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

        let parent_path = parent_path.ok_or(crate::error::ForumError::CommentMissing)?;

        // Find max child path under this parent inside the locked transaction.
        let max_child: String = sqlx::query_scalar(
            "SELECT COALESCE(MAX(path), '') FROM forum.comments \
             WHERE thread_id = $1 AND parent_id = $2 AND path IS NOT NULL",
        )
        .bind(thread_id)
        .bind(pid)
        .fetch_one(&mut *tx)
        .await?;

        let next_index = next_sibling_index(&max_child, &parent_path);
        let path = format!("{parent_path}.{next_index:04x}");

        insert_comment_tx(
            &mut tx,
            NewComment {
                thread_id,
                parent_id,
                path: &path,
                author_id,
                body: source.body,
                content_format: source.content_format,
                quoted_comment_id,
                is_hidden,
            },
        )
        .await?
    } else {
        // Top-level comment: find next top-level index.
        let max_path: String = sqlx::query_scalar(
            "SELECT COALESCE(MAX(path), '') FROM forum.comments \
             WHERE thread_id = $1 AND parent_id IS NULL AND path IS NOT NULL",
        )
        .bind(thread_id)
        .fetch_one(&mut *tx)
        .await?;

        let top_level = next_sibling_index(&max_path, "");
        let path = format!("{top_level:04x}");

        insert_comment_tx(
            &mut tx,
            NewComment {
                thread_id,
                parent_id: None,
                path: &path,
                author_id,
                body: source.body,
                content_format: source.content_format,
                quoted_comment_id,
                is_hidden,
            },
        )
        .await?
    };
    media::attachments::sync_forum_asset_bindings(
        &mut tx,
        author_id,
        media::attachments::ForumTargetType::Comment,
        row.id,
        row.content_version,
        source.image_references,
    )
    .await?;
    if !is_hidden {
        let body_excerpt = crate::content_policy::plain_text_projection(
            &row.body,
            crate::dto::ContentFormat::from_db(&row.content_format),
            100,
        );
        platform::outbox::enqueue_achievement_award_tx(
            &mut tx,
            &format!("forum-comment:{}:achievement:first-comment", row.id),
            author_id,
            author_id,
            "first-comment",
            "published a first forum comment",
        )
        .await?;
        if let Some(thread_author_id) =
            thread_state.author_id.filter(|target_id| *target_id != author_id)
        {
            platform::outbox::enqueue_notification_tx(
                &mut tx,
                &format!("forum-comment:{}:reply:{thread_author_id}", row.id),
                thread_author_id,
                Some(author_id),
                "reply",
                &serde_json::json!({
                    "threadId": thread_id.to_string(),
                    "commentId": row.id.to_string(),
                    "authorHandle": &row.author_handle,
                    "bodyExcerpt": &body_excerpt,
                    "title": format!("{} 回复了你的主题", row.author_handle),
                }),
                None,
                None,
            )
            .await?;
        }
        if let Some(parent_author_id) = parent_author_id.filter(|target_id| {
            *target_id != author_id && Some(*target_id) != thread_state.author_id
        }) {
            platform::outbox::enqueue_notification_tx(
                &mut tx,
                &format!("forum-comment:{}:parent-reply:{parent_author_id}", row.id),
                parent_author_id,
                Some(author_id),
                "reply",
                &serde_json::json!({
                    "threadId": thread_id.to_string(),
                    "commentId": row.id.to_string(),
                    "authorHandle": &row.author_handle,
                    "bodyExcerpt": &body_excerpt,
                    "title": format!("{} 回复了你的评论", row.author_handle),
                }),
                None,
                None,
            )
            .await?;
        }
        if let (Some(quoted_comment_id), Some(quoted_author_id)) =
            (quoted_comment_id, quoted_author_id.filter(|target_id| *target_id != author_id))
        {
            platform::outbox::enqueue_notification_tx(
                &mut tx,
                &format!("forum-comment:{}:quote:{quoted_author_id}", row.id),
                quoted_author_id,
                Some(author_id),
                "quote",
                &serde_json::json!({
                    "threadId": thread_id.to_string(),
                    "commentId": row.id.to_string(),
                    "quotedCommentId": quoted_comment_id.to_string(),
                    "authorHandle": &row.author_handle,
                    "bodyExcerpt": &body_excerpt,
                    "title": format!("{} 引用了你的评论", row.author_handle),
                }),
                None,
                None,
            )
            .await?;
        }

        let mut watcher_exclude = vec![author_id];
        watcher_exclude.extend(
            [thread_state.author_id, parent_author_id, quoted_author_id].into_iter().flatten(),
        );
        watcher_exclude.sort_unstable();
        watcher_exclude.dedup();
        for watcher_id in super::subscriptions::get_watching_subscriber_ids_tx(
            &mut tx,
            thread_id,
            &watcher_exclude,
        )
        .await?
        {
            platform::outbox::enqueue_notification_tx(
                &mut tx,
                &format!("forum-comment:{}:watching:{watcher_id}", row.id),
                watcher_id,
                Some(author_id),
                "watching",
                &serde_json::json!({
                    "threadId": thread_id.to_string(),
                    "commentId": row.id.to_string(),
                    "authorHandle": &row.author_handle,
                    "bodyExcerpt": &body_excerpt,
                    "title": "你订阅的主题有新回复",
                }),
                Some(&format!("watching:{thread_id}")),
                None,
            )
            .await?;
        }

        let mention_handles = crate::content_policy::mention_handles(
            &row.body,
            crate::dto::ContentFormat::from_db(&row.content_format),
            &row.author_handle,
        );
        for target in
            identity::public_accounts::find_mention_targets_by_handles_tx(&mut tx, &mention_handles)
                .await?
                .into_iter()
                .filter(|target| target.id != author_id)
        {
            platform::outbox::enqueue_notification_tx(
                &mut tx,
                &format!("forum-comment:{}:mention:{}", row.id, target.id),
                target.id,
                Some(author_id),
                "mention",
                &serde_json::json!({
                    "threadId": thread_id.to_string(),
                    "commentId": row.id.to_string(),
                    "authorHandle": &row.author_handle,
                    "handle": target.handle,
                    "bodyExcerpt": &body_excerpt,
                    "title": format!("{} 提及了你", row.author_handle),
                }),
                None,
                None,
            )
            .await?;
        }
    }
    tx.commit().await?;
    Ok(CommentCreateOutcome { row, thread_author_id: thread_state.author_id, quoted_author_id })
}

/// Insert the comment row and update thread reply_count in the active transaction.
async fn insert_comment_tx(
    tx: &mut PgConnection,
    comment: NewComment<'_>,
) -> AppResult<CommentRowJoined> {
    let row = sqlx::query_as::<_, CommentRowJoined>(
        "WITH inserted AS ( \
            INSERT INTO forum.comments (thread_id, parent_id, path, author_id, body, content_format, quoted_comment_id, hidden_at) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, CASE WHEN $8 THEN now() ELSE NULL END) \
            RETURNING id, thread_id, parent_id, path, author_id, body, content_format, content_version, vote_count, deleted_at, hidden_at, edited_at, created_at, quoted_comment_id \
         ) \
         SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                c.body, c.content_format, c.content_version, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                c.quoted_comment_id, \
                a.handle AS author_handle \
         FROM inserted c \
         JOIN identity.accounts a ON a.id = c.author_id",
    )
    .bind(comment.thread_id)
    .bind(comment.parent_id)
    .bind(comment.path)
    .bind(comment.author_id)
    .bind(comment.body)
    .bind(comment.content_format)
    .bind(comment.quoted_comment_id)
    .bind(comment.is_hidden)
    .fetch_one(&mut *tx)
    .await?;

    if !comment.is_hidden {
        super::subscriptions::lock_account_subscriptions(tx, comment.author_id).await?;
        sqlx::query(
            "UPDATE forum.threads \
             SET reply_count = reply_count + 1, last_activity_at = now() \
             WHERE id = $1",
        )
        .bind(comment.thread_id)
        .execute(&mut *tx)
        .await?;
        activity::contributions::activate_contribution(
            tx,
            comment.author_id,
            activity::contributions::ActivityKind::Comment,
            &format!("forum_comment:{}", row.id),
            row.created_at,
        )
        .await?;
        sqlx::query(
            "INSERT INTO forum.user_stats (account_id, comments_created, last_posted_at) \
             VALUES ($1, 1, now()) \
             ON CONFLICT (account_id) DO UPDATE \
             SET comments_created = forum.user_stats.comments_created + 1, \
                 last_posted_at = now(), updated_at = now()",
        )
        .bind(comment.author_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO forum.subscriptions (account_id, target_type, target_id, level) \
             VALUES ($1, 'thread', $2, 'tracking') \
             ON CONFLICT (account_id, target_type, target_id) \
             DO UPDATE SET level = EXCLUDED.level",
        )
        .bind(comment.author_id)
        .bind(comment.thread_id)
        .execute(&mut *tx)
        .await?;
    }

    Ok(row)
}

/// Compute the next sibling index from the max child path.
///
/// Given a max child path like "0003.0007" and parent path "0003", returns 8.
pub fn next_sibling_index(max_child_path: &str, parent_path: &str) -> u32 {
    if max_child_path.is_empty() || max_child_path == parent_path {
        1
    } else {
        let parent_prefix =
            if parent_path.is_empty() { String::new() } else { format!("{parent_path}.") };
        let suffix = max_child_path.strip_prefix(&parent_prefix).unwrap_or(max_child_path);
        let last = suffix.split('.').next().unwrap_or("0");
        u32::from_str_radix(last, 16).unwrap_or(0).saturating_add(1)
    }
}

/// Find a single comment by id, joined with author handle.
pub async fn find_comment(pool: &PgPool, id: i64) -> AppResult<Option<CommentRowJoined>> {
    let row = sqlx::query_as::<_, CommentRowJoined>(
        "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                c.body, c.content_format, c.content_version, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                c.quoted_comment_id, \
                a.handle AS author_handle \
         FROM forum.comments c \
         JOIN identity.accounts a ON a.id = c.author_id \
         WHERE c.id = $1 AND c.deleted_at IS NULL AND c.hidden_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Find a comment for staff recovery, including hidden and soft-deleted rows.
pub async fn find_comment_for_moderation(
    pool: &PgPool,
    id: i64,
) -> AppResult<Option<CommentRowJoined>> {
    let row = sqlx::query_as::<_, CommentRowJoined>(
        "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                c.body, c.content_format, c.content_version, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                c.quoted_comment_id, a.handle AS author_handle \
         FROM forum.comments c \
         JOIN identity.accounts a ON a.id = c.author_id \
         WHERE c.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Update an available comment and its revision atomically.
pub async fn update_comment(
    pool: &PgPool,
    id: i64,
    author_id: i64,
    source: CommentUpdateSource<'_>,
) -> AppResult<CommentRowJoined> {
    let mut tx = pool.begin().await?;
    let thread_id: i64 = sqlx::query_scalar("SELECT thread_id FROM forum.comments WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(shared::AppError::NotFound)?;
    let thread_state: ThreadPostingState = sqlx::query_as(
        "SELECT author_id, deleted_at, hidden_at, closed_at, archived_at \
         FROM forum.threads WHERE id = $1 FOR UPDATE",
    )
    .bind(thread_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(shared::AppError::NotFound)?;
    if thread_state.deleted_at.is_some() || thread_state.hidden_at.is_some() {
        return Err(shared::AppError::NotFound);
    }
    if thread_state.archived_at.is_some() {
        return Err(shared::AppError::Conflict("thread is archived".into()));
    }

    let existing = sqlx::query_as::<_, CommentRowJoined>(
        "SELECT c.id, c.thread_id, c.parent_id, c.path, c.author_id, \
                c.body, c.content_format, c.content_version, c.vote_count, c.deleted_at, c.hidden_at, c.edited_at, c.created_at, \
                c.quoted_comment_id, a.handle AS author_handle \
         FROM forum.comments c \
         JOIN identity.accounts a ON a.id = c.author_id \
         WHERE c.id = $1 AND c.thread_id = $2 FOR UPDATE OF c",
    )
    .bind(id)
    .bind(thread_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(shared::AppError::NotFound)?;
    if existing.author_id != author_id {
        return Err(shared::AppError::Forbidden);
    }
    if existing.deleted_at.is_some() || existing.hidden_at.is_some() {
        return Err(shared::AppError::NotFound);
    }
    if source.expected_version != existing.content_version {
        return Err(shared::AppError::OptimisticLockConflict {
            current_version: existing.content_version,
        });
    }

    let body_changed =
        existing.body != source.body || existing.content_format != source.content_format;
    let within_grace = existing.created_at > chrono::Utc::now() - chrono::Duration::minutes(5);
    if body_changed && !within_grace {
        super::revisions::create_revision_tx(
            &mut tx,
            "comment",
            id,
            author_id,
            super::revisions::RevisionSource {
                old_title: None,
                old_body: &existing.body,
                old_content_format: &existing.content_format,
                old_content_version: existing.content_version,
            },
        )
        .await?;
    }

    let row = sqlx::query_as::<_, CommentRowJoined>(
        "WITH updated AS ( \
         UPDATE forum.comments SET \
           body = $1, \
           content_format = $2, \
           content_version = content_version + 1, \
           edited_at = CASE WHEN $3 THEN now() ELSE edited_at END, \
           hidden_at = CASE WHEN $4 THEN now() ELSE hidden_at END \
         WHERE id = $5 AND content_version = $6 \
         RETURNING id, thread_id, parent_id, path, author_id, body, content_format, content_version, vote_count, \
                   deleted_at, hidden_at, edited_at, created_at, quoted_comment_id \
         ) \
         SELECT u.id, u.thread_id, u.parent_id, u.path, u.author_id, \
                u.body, u.content_format, u.content_version, u.vote_count, u.deleted_at, u.hidden_at, u.edited_at, u.created_at, \
                u.quoted_comment_id, \
                a.handle AS author_handle \
         FROM updated u \
         JOIN identity.accounts a ON a.id = u.author_id",
    )
    .bind(source.body)
    .bind(source.content_format)
    .bind(body_changed)
    .bind(source.is_queued)
    .bind(id)
    .bind(source.expected_version)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(shared::AppError::OptimisticLockConflict {
        current_version: existing.content_version,
    })?;

    media::attachments::sync_forum_asset_bindings(
        &mut tx,
        author_id,
        media::attachments::ForumTargetType::Comment,
        id,
        row.content_version,
        source.image_references,
    )
    .await?;

    if source.is_queued {
        sqlx::query(
            "UPDATE forum.threads SET reply_count = GREATEST(reply_count - 1, 0) WHERE id = $1",
        )
        .bind(thread_id)
        .execute(&mut *tx)
        .await?;
        super::activity_projection::synchronize_comment_activity(&mut tx, id, chrono::Utc::now())
            .await?;
    }

    tx.commit().await?;
    Ok(row)
}
