//! Database access layer for the forum domain.
//!
//! Every function takes `&PgPool` and returns `Result` so the caller
//! (typically a handler) can use `?` and let Axum render errors.

// ---------------------------------------------------------------------------
// cursor helpers
// ---------------------------------------------------------------------------

pub fn base64_encode_i64(val: i64) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(val.to_string())
}

pub(crate) fn base64_decode_i64(s: &str) -> Result<i64, String> {
    use base64::Engine;
    let bytes =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).map_err(|e| e.to_string())?;
    let s = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    s.parse::<i64>().map_err(|e| e.to_string())
}

pub(crate) fn base64_encode_str(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s)
}

pub(crate) fn base64_decode_str(s: &str) -> Result<String, String> {
    use base64::Engine;
    let bytes =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

pub(crate) fn encode_hot_cursor(hot_score: f64, id: i64) -> String {
    base64_encode_str(&format!("{hot_score}:{id}"))
}

pub(crate) fn decode_hot_cursor(cursor: &str) -> Result<(f64, i64), String> {
    let s = base64_decode_str(cursor)?;
    let (hot_str, id_str) = s.rsplit_once(':').ok_or("invalid hot cursor")?;
    let hot_score = hot_str.parse::<f64>().map_err(|e| e.to_string())?;
    let id = id_str.parse::<i64>().map_err(|e| e.to_string())?;
    Ok((hot_score, id))
}

pub mod boards;
pub mod comments;
pub mod hot_rank;
pub mod threads;
pub mod votes;

pub mod bookmarks;
pub mod dms;
pub mod drafts;
pub mod flags;
pub mod ignores;
pub mod mod_actions;
pub mod notification_prefs;
pub mod notifications;
pub mod polls;
pub mod profiles;
pub mod reads;
pub mod revisions;
pub mod subscriptions;
pub mod tags;
pub mod thread_state;

pub use flags::{insert_flag, list_flag_queue, resolve_flag};
pub use mod_actions::{insert_mod_action, list_mod_actions};

pub use boards::{find_board, list_boards};
pub use bookmarks::{delete_bookmark, list_bookmarks, upsert_bookmark};
pub use comments::{
    create_comment, find_comment, find_comment_for_moderation, list_comments, next_sibling_index,
    update_comment,
};
pub use drafts::{
    count_drafts, delete_draft, delete_drafts_for_account, draft_exists, get_draft, list_drafts,
    upsert_draft,
};
pub use hot_rank::refresh_hot_rank;
pub use ignores::{
    batch_ignored_ids, delete_ignore, insert_ignore, is_ignored, list_ignored_ids,
    list_ignored_users,
};
pub use notification_prefs::{get_notification_prefs, set_notification_prefs};
pub use notifications::{list_notifications, mark_all_read, mark_read, NotificationRow};
pub use polls::{
    create_poll, get_poll, get_poll_by_id, get_poll_id_by_thread, get_poll_option,
    get_poll_results, get_voted_option_ids, vote_option, PollWithOptions,
};
pub use reads::{get_last_read_comment_id, get_unread_thread_ids, upsert_read_position};
pub use revisions::{create_revision, list_revisions};
pub use subscriptions::{
    delete_subscription, get_following_thread_ids, get_muted_ids, get_thread_subscription,
    get_watching_subscriber_ids, list_subscriptions, set_subscription,
};
pub use tags::{
    create_tag, delete_tag, find_tag, find_tag_by_slug, get_thread_tag_slugs, list_tags,
    resolve_tag_slugs, set_thread_tags, update_tag,
};
pub use thread_state::{
    archive_thread, auto_archive_stale, clear_solved_answer, close_thread, hide_thread,
    move_thread, pin_thread, reopen_thread, restore_thread, set_solved_answer, unarchive_thread,
    unhide_thread, unpin_thread,
};
pub use threads::{
    create_thread, fetch_threads_by_ids, find_thread, find_thread_for_moderation, list_threads,
    list_threads_feed, list_threads_feed_following, update_thread,
};
pub use votes::{
    deactivate_target_vote_contributions, reactivate_target_vote_contributions, vote_post,
};
