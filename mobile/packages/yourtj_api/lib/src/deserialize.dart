import 'package:yourtj_api/src/model/account.dart';
import 'package:yourtj_api/src/model/account_data_export.dart';
import 'package:yourtj_api/src/model/account_lifecycle.dart';
import 'package:yourtj_api/src/model/account_lifecycle_mutation.dart';
import 'package:yourtj_api/src/model/account_lifecycle_mutation_input.dart';
import 'package:yourtj_api/src/model/achievement.dart';
import 'package:yourtj_api/src/model/achievement_create_input.dart';
import 'package:yourtj_api/src/model/achievement_event.dart';
import 'package:yourtj_api/src/model/achievement_event_page.dart';
import 'package:yourtj_api/src/model/achievement_grant.dart';
import 'package:yourtj_api/src/model/achievement_grant_input.dart';
import 'package:yourtj_api/src/model/achievement_grant_page.dart';
import 'package:yourtj_api/src/model/achievement_page.dart';
import 'package:yourtj_api/src/model/achievement_revoke_input.dart';
import 'package:yourtj_api/src/model/achievement_update_input.dart';
import 'package:yourtj_api/src/model/activity_calendar.dart';
import 'package:yourtj_api/src/model/activity_day.dart';
import 'package:yourtj_api/src/model/activity_policy.dart';
import 'package:yourtj_api/src/model/activity_policy_page.dart';
import 'package:yourtj_api/src/model/activity_policy_update_input.dart';
import 'package:yourtj_api/src/model/activity_weights.dart';
import 'package:yourtj_api/src/model/admin_appeal.dart';
import 'package:yourtj_api/src/model/admin_appeal_page.dart';
import 'package:yourtj_api/src/model/admin_audit_event.dart';
import 'package:yourtj_api/src/model/admin_audit_event_page.dart';
import 'package:yourtj_api/src/model/admin_board_create_input.dart';
import 'package:yourtj_api/src/model/admin_board_update_input.dart';
import 'package:yourtj_api/src/model/admin_comment_action_input.dart';
import 'package:yourtj_api/src/model/admin_course_create_input.dart';
import 'package:yourtj_api/src/model/admin_course_update_input.dart';
import 'package:yourtj_api/src/model/admin_forum_flag.dart';
import 'package:yourtj_api/src/model/admin_forum_flag_page.dart';
import 'package:yourtj_api/src/model/admin_lifecycle_job.dart';
import 'package:yourtj_api/src/model/admin_lifecycle_job_page.dart';
import 'package:yourtj_api/src/model/admin_overview.dart';
import 'package:yourtj_api/src/model/admin_reason_input.dart';
import 'package:yourtj_api/src/model/admin_tag_create_input.dart';
import 'package:yourtj_api/src/model/admin_tag_update_input.dart';
import 'package:yourtj_api/src/model/admin_thread_action_input.dart';
import 'package:yourtj_api/src/model/admin_user.dart';
import 'package:yourtj_api/src/model/admin_user_invite_input.dart';
import 'package:yourtj_api/src/model/admin_user_page.dart';
import 'package:yourtj_api/src/model/admin_user_role_input.dart';
import 'package:yourtj_api/src/model/admin_versioned_archive_input.dart';
import 'package:yourtj_api/src/model/ai_summary.dart';
import 'package:yourtj_api/src/model/announcement.dart';
import 'package:yourtj_api/src/model/announcement_create_input.dart';
import 'package:yourtj_api/src/model/announcement_page.dart';
import 'package:yourtj_api/src/model/announcement_receipt.dart';
import 'package:yourtj_api/src/model/announcement_receipt_input.dart';
import 'package:yourtj_api/src/model/announcement_receipt_summary.dart';
import 'package:yourtj_api/src/model/announcement_revision.dart';
import 'package:yourtj_api/src/model/announcement_revision_page.dart';
import 'package:yourtj_api/src/model/announcement_update_input.dart';
import 'package:yourtj_api/src/model/appeal.dart';
import 'package:yourtj_api/src/model/appeal_access_token.dart';
import 'package:yourtj_api/src/model/appeal_decision_input.dart';
import 'package:yourtj_api/src/model/appeal_email_verification.dart';
import 'package:yourtj_api/src/model/appeal_history.dart';
import 'package:yourtj_api/src/model/appeal_page.dart';
import 'package:yourtj_api/src/model/appeal_transition_input.dart';
import 'package:yourtj_api/src/model/auth_appeal_password_post_request.dart';
import 'package:yourtj_api/src/model/auth_password_forgot_post_request.dart';
import 'package:yourtj_api/src/model/auth_password_login_post_request.dart';
import 'package:yourtj_api/src/model/auth_recovery_email_verify_post_request.dart';
import 'package:yourtj_api/src/model/auth_tokens.dart';
import 'package:yourtj_api/src/model/board.dart';
import 'package:yourtj_api/src/model/board_search_hit.dart';
import 'package:yourtj_api/src/model/bookmark.dart';
import 'package:yourtj_api/src/model/bookmark_input.dart';
import 'package:yourtj_api/src/model/bookmark_page.dart';
import 'package:yourtj_api/src/model/calendar.dart';
import 'package:yourtj_api/src/model/campus.dart';
import 'package:yourtj_api/src/model/check_in_status.dart';
import 'package:yourtj_api/src/model/comment.dart';
import 'package:yourtj_api/src/model/comment_draft_payload.dart';
import 'package:yourtj_api/src/model/comment_input.dart';
import 'package:yourtj_api/src/model/comment_page.dart';
import 'package:yourtj_api/src/model/comment_update_input.dart';
import 'package:yourtj_api/src/model/course.dart';
import 'package:yourtj_api/src/model/course_detail.dart';
import 'package:yourtj_api/src/model/course_nature.dart';
import 'package:yourtj_api/src/model/course_page.dart';
import 'package:yourtj_api/src/model/course_search_hit.dart';
import 'package:yourtj_api/src/model/create_review_input.dart';
import 'package:yourtj_api/src/model/credit_reconciliation_run.dart';
import 'package:yourtj_api/src/model/credit_reconciliation_run_page.dart';
import 'package:yourtj_api/src/model/credit_reconciliation_stats.dart';
import 'package:yourtj_api/src/model/credit_reconciliation_wallet.dart';
import 'package:yourtj_api/src/model/credit_reconciliation_wallet_page.dart';
import 'package:yourtj_api/src/model/data_export_download_grant.dart';
import 'package:yourtj_api/src/model/data_export_job.dart';
import 'package:yourtj_api/src/model/deactivate_account_input.dart';
import 'package:yourtj_api/src/model/delete_account_input.dart';
import 'package:yourtj_api/src/model/department.dart';
import 'package:yourtj_api/src/model/dm_conversation.dart';
import 'package:yourtj_api/src/model/dm_conversation_input.dart';
import 'package:yourtj_api/src/model/dm_conversation_page.dart';
import 'package:yourtj_api/src/model/dm_counts.dart';
import 'package:yourtj_api/src/model/dm_message.dart';
import 'package:yourtj_api/src/model/dm_message_input.dart';
import 'package:yourtj_api/src/model/dm_message_page.dart';
import 'package:yourtj_api/src/model/dm_read_input.dart';
import 'package:yourtj_api/src/model/dm_report.dart';
import 'package:yourtj_api/src/model/dm_report_input.dart';
import 'package:yourtj_api/src/model/dm_report_page.dart';
import 'package:yourtj_api/src/model/dm_report_resolution_input.dart';
import 'package:yourtj_api/src/model/draft_output.dart';
import 'package:yourtj_api/src/model/draft_page.dart';
import 'package:yourtj_api/src/model/draft_save_input.dart';
import 'package:yourtj_api/src/model/email_code_request.dart';
import 'package:yourtj_api/src/model/email_code_verification.dart';
import 'package:yourtj_api/src/model/email_notification_prefs.dart';
import 'package:yourtj_api/src/model/error.dart';
import 'package:yourtj_api/src/model/error_error.dart';
import 'package:yourtj_api/src/model/faculty.dart';
import 'package:yourtj_api/src/model/feature_thread_input.dart';
import 'package:yourtj_api/src/model/featured_thread_response.dart';
import 'package:yourtj_api/src/model/flag_input.dart';
import 'package:yourtj_api/src/model/flag_resolve_input.dart';
import 'package:yourtj_api/src/model/flag_response.dart';
import 'package:yourtj_api/src/model/forum_attachment.dart';
import 'package:yourtj_api/src/model/forum_draft_payload.dart';
import 'package:yourtj_api/src/model/forum_polls_id_vote_post_request.dart';
import 'package:yourtj_api/src/model/forum_threads_id_delete200_response.dart';
import 'package:yourtj_api/src/model/governance_notice.dart';
import 'package:yourtj_api/src/model/governance_notice_page.dart';
import 'package:yourtj_api/src/model/governance_notice_read_input.dart';
import 'package:yourtj_api/src/model/health_status.dart';
import 'package:yourtj_api/src/model/ignore_page.dart';
import 'package:yourtj_api/src/model/ignore_user.dart';
import 'package:yourtj_api/src/model/in_app_notification_prefs.dart';
import 'package:yourtj_api/src/model/in_app_notification_prefs_input.dart';
import 'package:yourtj_api/src/model/latest_update.dart';
import 'package:yourtj_api/src/model/ledger_entry.dart';
import 'package:yourtj_api/src/model/ledger_page.dart';
import 'package:yourtj_api/src/model/ledger_verify.dart';
import 'package:yourtj_api/src/model/major.dart';
import 'package:yourtj_api/src/model/me_governance_notices_unread_count_get200_response.dart';
import 'package:yourtj_api/src/model/me_patch_request.dart';
import 'package:yourtj_api/src/model/media_deletion_job.dart';
import 'package:yourtj_api/src/model/media_deletion_job_page.dart';
import 'package:yourtj_api/src/model/media_delivery.dart';
import 'package:yourtj_api/src/model/media_moderation_input.dart';
import 'package:yourtj_api/src/model/media_provider_inventory_status.dart';
import 'package:yourtj_api/src/model/media_reconciliation_finding.dart';
import 'package:yourtj_api/src/model/media_reconciliation_report.dart';
import 'package:yourtj_api/src/model/media_retention_hold.dart';
import 'package:yourtj_api/src/model/media_retention_hold_input.dart';
import 'package:yourtj_api/src/model/media_retention_hold_page.dart';
import 'package:yourtj_api/src/model/media_retention_hold_release_input.dart';
import 'package:yourtj_api/src/model/mod_action.dart';
import 'package:yourtj_api/src/model/mod_action_page.dart';
import 'package:yourtj_api/src/model/moderation_preview_grant.dart';
import 'package:yourtj_api/src/model/my_profile.dart';
import 'package:yourtj_api/src/model/my_upload.dart';
import 'package:yourtj_api/src/model/my_upload_page.dart';
import 'package:yourtj_api/src/model/notification.dart';
import 'package:yourtj_api/src/model/notification_outbox_event.dart';
import 'package:yourtj_api/src/model/notification_outbox_event_page.dart';
import 'package:yourtj_api/src/model/notification_outbox_retry_input.dart';
import 'package:yourtj_api/src/model/notification_page.dart';
import 'package:yourtj_api/src/model/notification_preferences.dart';
import 'package:yourtj_api/src/model/notification_preferences_input.dart';
import 'package:yourtj_api/src/model/notification_prefs.dart';
import 'package:yourtj_api/src/model/notification_prefs_input.dart';
import 'package:yourtj_api/src/model/notification_read_input.dart';
import 'package:yourtj_api/src/model/notification_unread_count.dart';
import 'package:yourtj_api/src/model/onboarding_complete_input.dart';
import 'package:yourtj_api/src/model/onboarding_state.dart';
import 'package:yourtj_api/src/model/onebox_result.dart';
import 'package:yourtj_api/src/model/page.dart';
import 'package:yourtj_api/src/model/password_change_input.dart';
import 'package:yourtj_api/src/model/password_reset_input.dart';
import 'package:yourtj_api/src/model/password_set_input.dart';
import 'package:yourtj_api/src/model/poll.dart';
import 'package:yourtj_api/src/model/poll_input.dart';
import 'package:yourtj_api/src/model/poll_option.dart';
import 'package:yourtj_api/src/model/poll_vote_response.dart';
import 'package:yourtj_api/src/model/post_revision.dart';
import 'package:yourtj_api/src/model/product.dart';
import 'package:yourtj_api/src/model/product_input.dart';
import 'package:yourtj_api/src/model/product_page.dart';
import 'package:yourtj_api/src/model/profile_asset_input.dart';
import 'package:yourtj_api/src/model/profile_content.dart';
import 'package:yourtj_api/src/model/profile_content_page.dart';
import 'package:yourtj_api/src/model/profile_privacy.dart';
import 'package:yourtj_api/src/model/profile_privacy_update_input.dart';
import 'package:yourtj_api/src/model/profile_update_input.dart';
import 'package:yourtj_api/src/model/promotion.dart';
import 'package:yourtj_api/src/model/promotion_create_input.dart';
import 'package:yourtj_api/src/model/promotion_event_input.dart';
import 'package:yourtj_api/src/model/promotion_metric_day.dart';
import 'package:yourtj_api/src/model/promotion_metric_summary.dart';
import 'package:yourtj_api/src/model/promotion_metrics.dart';
import 'package:yourtj_api/src/model/promotion_page.dart';
import 'package:yourtj_api/src/model/promotion_update_input.dart';
import 'package:yourtj_api/src/model/public_verification.dart';
import 'package:yourtj_api/src/model/purchase.dart';
import 'package:yourtj_api/src/model/purchase_action.dart';
import 'package:yourtj_api/src/model/purchase_page.dart';
import 'package:yourtj_api/src/model/read_tracking_input.dart';
import 'package:yourtj_api/src/model/recent_auth_status.dart';
import 'package:yourtj_api/src/model/recent_auth_verify_input.dart';
import 'package:yourtj_api/src/model/reconciliation_run_input.dart';
import 'package:yourtj_api/src/model/recovery_credential.dart';
import 'package:yourtj_api/src/model/refresh_input.dart';
import 'package:yourtj_api/src/model/report.dart';
import 'package:yourtj_api/src/model/report_page.dart';
import 'package:yourtj_api/src/model/review.dart';
import 'package:yourtj_api/src/model/review_input.dart';
import 'package:yourtj_api/src/model/review_page.dart';
import 'package:yourtj_api/src/model/review_report_resolution_input.dart';
import 'package:yourtj_api/src/model/review_search_hit.dart';
import 'package:yourtj_api/src/model/reviews_id_report_post_request.dart';
import 'package:yourtj_api/src/model/revision_page.dart';
import 'package:yourtj_api/src/model/sanction.dart';
import 'package:yourtj_api/src/model/sanction_input.dart';
import 'package:yourtj_api/src/model/search_highlight.dart';
import 'package:yourtj_api/src/model/search_highlight_range.dart';
import 'package:yourtj_api/src/model/search_result.dart';
import 'package:yourtj_api/src/model/selection_course.dart';
import 'package:yourtj_api/src/model/selection_offering.dart';
import 'package:yourtj_api/src/model/selection_offering_page.dart';
import 'package:yourtj_api/src/model/selection_sync_job.dart';
import 'package:yourtj_api/src/model/selection_sync_job_page.dart';
import 'package:yourtj_api/src/model/session.dart';
import 'package:yourtj_api/src/model/session_page.dart';
import 'package:yourtj_api/src/model/setting.dart';
import 'package:yourtj_api/src/model/setting_update_input.dart';
import 'package:yourtj_api/src/model/signing_intent.dart';
import 'package:yourtj_api/src/model/signing_intent_input.dart';
import 'package:yourtj_api/src/model/signing_intent_outcome.dart';
import 'package:yourtj_api/src/model/signing_intent_outcome_input.dart';
import 'package:yourtj_api/src/model/startup_verify_post_request.dart';
import 'package:yourtj_api/src/model/submit_appeal_input.dart';
import 'package:yourtj_api/src/model/subscription.dart';
import 'package:yourtj_api/src/model/subscription_input.dart';
import 'package:yourtj_api/src/model/subscription_page.dart';
import 'package:yourtj_api/src/model/tag.dart';
import 'package:yourtj_api/src/model/tag_search_hit.dart';
import 'package:yourtj_api/src/model/task.dart';
import 'package:yourtj_api/src/model/task_action.dart';
import 'package:yourtj_api/src/model/task_input.dart';
import 'package:yourtj_api/src/model/task_page.dart';
import 'package:yourtj_api/src/model/teacher.dart';
import 'package:yourtj_api/src/model/thread.dart';
import 'package:yourtj_api/src/model/thread_detail.dart';
import 'package:yourtj_api/src/model/thread_draft_payload.dart';
import 'package:yourtj_api/src/model/thread_feed.dart';
import 'package:yourtj_api/src/model/thread_feed_page.dart';
import 'package:yourtj_api/src/model/thread_input.dart';
import 'package:yourtj_api/src/model/thread_move_input.dart';
import 'package:yourtj_api/src/model/thread_page.dart';
import 'package:yourtj_api/src/model/thread_pin_input.dart';
import 'package:yourtj_api/src/model/thread_search_hit.dart';
import 'package:yourtj_api/src/model/thread_update_input.dart';
import 'package:yourtj_api/src/model/time_slot.dart';
import 'package:yourtj_api/src/model/tip_input.dart';
import 'package:yourtj_api/src/model/trust_level_adjust_input.dart';
import 'package:yourtj_api/src/model/trust_level_event.dart';
import 'package:yourtj_api/src/model/trust_level_event_page.dart';
import 'package:yourtj_api/src/model/trust_level_policy.dart';
import 'package:yourtj_api/src/model/trust_level_policy_page.dart';
import 'package:yourtj_api/src/model/trust_level_policy_update_input.dart';
import 'package:yourtj_api/src/model/trust_progress.dart';
import 'package:yourtj_api/src/model/unsanction_input.dart';
import 'package:yourtj_api/src/model/unsubscribe_input.dart';
import 'package:yourtj_api/src/model/upload.dart';
import 'package:yourtj_api/src/model/upload_credentials.dart';
import 'package:yourtj_api/src/model/upload_intent_input.dart';
import 'package:yourtj_api/src/model/upload_page.dart';
import 'package:yourtj_api/src/model/user_badge.dart';
import 'package:yourtj_api/src/model/user_comment.dart';
import 'package:yourtj_api/src/model/user_comment_page.dart';
import 'package:yourtj_api/src/model/user_profile.dart';
import 'package:yourtj_api/src/model/user_profile_with_stats.dart';
import 'package:yourtj_api/src/model/user_relationship.dart';
import 'package:yourtj_api/src/model/user_search_hit.dart';
import 'package:yourtj_api/src/model/user_summary.dart';
import 'package:yourtj_api/src/model/user_summary_page.dart';
import 'package:yourtj_api/src/model/user_thread.dart';
import 'package:yourtj_api/src/model/user_thread_page.dart';
import 'package:yourtj_api/src/model/verification_grant.dart';
import 'package:yourtj_api/src/model/verification_grant_input.dart';
import 'package:yourtj_api/src/model/verification_grant_page.dart';
import 'package:yourtj_api/src/model/verification_revoke_input.dart';
import 'package:yourtj_api/src/model/verification_type.dart';
import 'package:yourtj_api/src/model/verification_type_input.dart';
import 'package:yourtj_api/src/model/verification_type_page.dart';
import 'package:yourtj_api/src/model/vote_input.dart';
import 'package:yourtj_api/src/model/vote_response.dart';
import 'package:yourtj_api/src/model/wallet.dart';
import 'package:yourtj_api/src/model/wallet_bind_post_request.dart';
import 'package:yourtj_api/src/model/wallet_claim_challenge.dart';
import 'package:yourtj_api/src/model/wallet_claim_post_request.dart';
import 'package:yourtj_api/src/model/watched_word.dart';
import 'package:yourtj_api/src/model/watched_word_input.dart';

final _regList = RegExp(r'^List<(.*)>$');
final _regSet = RegExp(r'^Set<(.*)>$');
final _regMap = RegExp(r'^Map<String,(.*)>$');

ReturnType deserialize<ReturnType, BaseType>(
  dynamic value,
  String targetType, {
  bool growable = true,
}) {
  switch (targetType) {
    case 'String':
      return '$value' as ReturnType;
    case 'int':
      return (value is int ? value : int.parse('$value')) as ReturnType;
    case 'bool':
      if (value is bool) {
        return value as ReturnType;
      }
      final valueString = '$value'.toLowerCase();
      return (valueString == 'true' || valueString == '1') as ReturnType;
    case 'double':
      return (value is double ? value : double.parse('$value')) as ReturnType;
    case 'Account':
      return Account.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'AccountDataExport':
      return AccountDataExport.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AccountLifecycle':
      return AccountLifecycle.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AccountLifecycleMutation':
      return AccountLifecycleMutation.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AccountLifecycleMutationInput':
      return AccountLifecycleMutationInput.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'AccountLifecycleState':
    case 'Achievement':
      return Achievement.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'AchievementCreateInput':
      return AchievementCreateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AchievementEvent':
      return AchievementEvent.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AchievementEventPage':
      return AchievementEventPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AchievementGrant':
      return AchievementGrant.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AchievementGrantInput':
      return AchievementGrantInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AchievementGrantPage':
      return AchievementGrantPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AchievementIcon':
    case 'AchievementPage':
      return AchievementPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AchievementRevokeInput':
      return AchievementRevokeInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AchievementStatus':
    case 'AchievementUpdateInput':
      return AchievementUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ActivityCalendar':
      return ActivityCalendar.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ActivityDay':
      return ActivityDay.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ActivityPolicy':
      return ActivityPolicy.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ActivityPolicyPage':
      return ActivityPolicyPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ActivityPolicyUpdateInput':
      return ActivityPolicyUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ActivityVisibility':
    case 'ActivityWeights':
      return ActivityWeights.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminAppeal':
      return AdminAppeal.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'AdminAppealPage':
      return AdminAppealPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminAuditEvent':
      return AdminAuditEvent.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminAuditEventPage':
      return AdminAuditEventPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminBoardCreateInput':
      return AdminBoardCreateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminBoardUpdateInput':
      return AdminBoardUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminCommentActionInput':
      return AdminCommentActionInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminCourseCreateInput':
      return AdminCourseCreateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminCourseUpdateInput':
      return AdminCourseUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminForumFlag':
      return AdminForumFlag.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminForumFlagPage':
      return AdminForumFlagPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminLifecycleJob':
      return AdminLifecycleJob.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminLifecycleJobPage':
      return AdminLifecycleJobPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminOverview':
      return AdminOverview.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminReasonInput':
      return AdminReasonInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminTagCreateInput':
      return AdminTagCreateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminTagUpdateInput':
      return AdminTagUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminThreadActionInput':
      return AdminThreadActionInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminUser':
      return AdminUser.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'AdminUserInviteInput':
      return AdminUserInviteInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminUserPage':
      return AdminUserPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminUserRoleInput':
      return AdminUserRoleInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AdminVersionedArchiveInput':
      return AdminVersionedArchiveInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AiSummary':
      return AiSummary.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Announcement':
      return Announcement.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'AnnouncementCreateInput':
      return AnnouncementCreateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AnnouncementPage':
      return AnnouncementPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AnnouncementReceipt':
      return AnnouncementReceipt.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AnnouncementReceiptInput':
      return AnnouncementReceiptInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AnnouncementReceiptSummary':
      return AnnouncementReceiptSummary.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AnnouncementRevision':
      return AnnouncementRevision.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AnnouncementRevisionPage':
      return AnnouncementRevisionPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AnnouncementUpdateInput':
      return AnnouncementUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Appeal':
      return Appeal.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'AppealAccessToken':
      return AppealAccessToken.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AppealDecisionInput':
      return AppealDecisionInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AppealEmailVerification':
      return AppealEmailVerification.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AppealHistory':
      return AppealHistory.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AppealPage':
      return AppealPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'AppealStatus':
    case 'AppealTransitionInput':
      return AppealTransitionInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'AuthAppealPasswordPostRequest':
      return AuthAppealPasswordPostRequest.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'AuthPasswordForgotPostRequest':
      return AuthPasswordForgotPostRequest.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'AuthPasswordLoginPostRequest':
      return AuthPasswordLoginPostRequest.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'AuthRecoveryEmailVerifyPostRequest':
      return AuthRecoveryEmailVerifyPostRequest.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'AuthTokens':
      return AuthTokens.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Board':
      return Board.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'BoardSearchHit':
      return BoardSearchHit.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Bookmark':
      return Bookmark.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'BookmarkInput':
      return BookmarkInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'BookmarkPage':
      return BookmarkPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Calendar':
      return Calendar.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Campus':
      return Campus.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'CheckInStatus':
      return CheckInStatus.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Comment':
      return Comment.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'CommentDraftPayload':
      return CommentDraftPayload.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'CommentInput':
      return CommentInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'CommentPage':
      return CommentPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'CommentUpdateInput':
      return CommentUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ContentFormat':
    case 'Course':
      return Course.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'CourseDetail':
      return CourseDetail.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'CourseNature':
      return CourseNature.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'CoursePage':
      return CoursePage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'CourseSearchHit':
      return CourseSearchHit.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'CreateReviewInput':
      return CreateReviewInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'CreditReconciliationRun':
      return CreditReconciliationRun.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'CreditReconciliationRunPage':
      return CreditReconciliationRunPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'CreditReconciliationStats':
      return CreditReconciliationStats.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'CreditReconciliationWallet':
      return CreditReconciliationWallet.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'CreditReconciliationWalletPage':
      return CreditReconciliationWalletPage.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'DataExportDownloadGrant':
      return DataExportDownloadGrant.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DataExportJob':
      return DataExportJob.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DataExportStatus':
    case 'DeactivateAccountInput':
      return DeactivateAccountInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DeleteAccountInput':
      return DeleteAccountInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Department':
      return Department.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'DmConversation':
      return DmConversation.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DmConversationInput':
      return DmConversationInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DmConversationPage':
      return DmConversationPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DmCounts':
      return DmCounts.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'DmMessage':
      return DmMessage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'DmMessageInput':
      return DmMessageInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DmMessagePage':
      return DmMessagePage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DmPolicy':
    case 'DmReadInput':
      return DmReadInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'DmReport':
      return DmReport.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'DmReportInput':
      return DmReportInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DmReportPage':
      return DmReportPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'DmReportResolutionInput':
      return DmReportResolutionInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'DraftOutput':
      return DraftOutput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'DraftPage':
      return DraftPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'DraftSaveInput':
      return DraftSaveInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'EmailCodePurpose':
    case 'EmailCodeRequest':
      return EmailCodeRequest.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'EmailCodeVerification':
      return EmailCodeVerification.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'EmailNotificationPrefs':
      return EmailNotificationPrefs.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Error':
      return Error.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ErrorError':
      return ErrorError.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Faculty':
      return Faculty.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'FeatureThreadInput':
      return FeatureThreadInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'FeaturedThreadResponse':
      return FeaturedThreadResponse.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'FlagInput':
      return FlagInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'FlagResolveInput':
      return FlagResolveInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'FlagResponse':
      return FlagResponse.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ForumAttachment':
      return ForumAttachment.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ForumDraftPayload':
      return ForumDraftPayload.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ForumPollsIdVotePostRequest':
      return ForumPollsIdVotePostRequest.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ForumThreadsIdDelete200Response':
      return ForumThreadsIdDelete200Response.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'GovernanceNotice':
      return GovernanceNotice.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'GovernanceNoticePage':
      return GovernanceNoticePage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'GovernanceNoticeReadInput':
      return GovernanceNoticeReadInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'HealthStatus':
      return HealthStatus.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'IgnorePage':
      return IgnorePage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'IgnoreUser':
      return IgnoreUser.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'InAppNotificationPrefs':
      return InAppNotificationPrefs.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'InAppNotificationPrefsInput':
      return InAppNotificationPrefsInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'LatestUpdate':
      return LatestUpdate.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'LedgerEntry':
      return LedgerEntry.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'LedgerPage':
      return LedgerPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'LedgerVerify':
      return LedgerVerify.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Major':
      return Major.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'MeGovernanceNoticesUnreadCountGet200Response':
      return MeGovernanceNoticesUnreadCountGet200Response.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'MePatchRequest':
      return MePatchRequest.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaDeletionJob':
      return MediaDeletionJob.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaDeletionJobPage':
      return MediaDeletionJobPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaDelivery':
      return MediaDelivery.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaDeliveryState':
    case 'MediaDeliveryVariant':
    case 'MediaModerationInput':
      return MediaModerationInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaProviderInventoryStatus':
      return MediaProviderInventoryStatus.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'MediaReconciliationFinding':
      return MediaReconciliationFinding.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaReconciliationIssueCode':
    case 'MediaReconciliationReport':
      return MediaReconciliationReport.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaRetentionHold':
      return MediaRetentionHold.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaRetentionHoldInput':
      return MediaRetentionHoldInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaRetentionHoldPage':
      return MediaRetentionHoldPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MediaRetentionHoldReleaseInput':
      return MediaRetentionHoldReleaseInput.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'MediaUsage':
    case 'MentionPolicy':
    case 'ModAction':
      return ModAction.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ModActionPage':
      return ModActionPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ModerationPreviewGrant':
      return ModerationPreviewGrant.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'MyProfile':
      return MyProfile.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'MyUpload':
      return MyUpload.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'MyUploadPage':
      return MyUploadPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Notification':
      return Notification.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'NotificationOutboxEvent':
      return NotificationOutboxEvent.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'NotificationOutboxEventPage':
      return NotificationOutboxEventPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'NotificationOutboxRetryInput':
      return NotificationOutboxRetryInput.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'NotificationPage':
      return NotificationPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'NotificationPreferences':
      return NotificationPreferences.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'NotificationPreferencesInput':
      return NotificationPreferencesInput.fromJson(
            value as Map<String, dynamic>,
          )
          as ReturnType;
    case 'NotificationPrefs':
      return NotificationPrefs.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'NotificationPrefsInput':
      return NotificationPrefsInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'NotificationReadInput':
      return NotificationReadInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'NotificationUnreadCount':
      return NotificationUnreadCount.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'OnboardingCompleteInput':
      return OnboardingCompleteInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'OnboardingState':
      return OnboardingState.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'OneboxResult':
      return OneboxResult.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Page':
      return Page.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'PasswordChangeInput':
      return PasswordChangeInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PasswordResetInput':
      return PasswordResetInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PasswordSetInput':
      return PasswordSetInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Poll':
      return Poll.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'PollInput':
      return PollInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'PollOption':
      return PollOption.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'PollVoteResponse':
      return PollVoteResponse.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PostRevision':
      return PostRevision.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Product':
      return Product.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ProductInput':
      return ProductInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ProductPage':
      return ProductPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ProfileAssetInput':
      return ProfileAssetInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ProfileContent':
      return ProfileContent.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ProfileContentPage':
      return ProfileContentPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ProfilePrivacy':
      return ProfilePrivacy.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ProfilePrivacyUpdateInput':
      return ProfilePrivacyUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ProfileUpdateInput':
      return ProfileUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ProfileVisibility':
    case 'Promotion':
      return Promotion.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'PromotionCreateInput':
      return PromotionCreateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PromotionEventInput':
      return PromotionEventInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PromotionMetricDay':
      return PromotionMetricDay.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PromotionMetricSummary':
      return PromotionMetricSummary.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PromotionMetrics':
      return PromotionMetrics.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PromotionPage':
      return PromotionPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PromotionUpdateInput':
      return PromotionUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PublicVerification':
      return PublicVerification.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Purchase':
      return Purchase.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'PurchaseAction':
      return PurchaseAction.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'PurchasePage':
      return PurchasePage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ReadTrackingInput':
      return ReadTrackingInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'RecentAuthMethod':
    case 'RecentAuthStatus':
      return RecentAuthStatus.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'RecentAuthVerifyInput':
      return RecentAuthVerifyInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ReconciliationRunInput':
      return ReconciliationRunInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'RecoveryCredential':
      return RecoveryCredential.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'RefreshInput':
      return RefreshInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'RelationshipListVisibility':
    case 'Report':
      return Report.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ReportPage':
      return ReportPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Review':
      return Review.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ReviewInput':
      return ReviewInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ReviewPage':
      return ReviewPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ReviewReportResolutionInput':
      return ReviewReportResolutionInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ReviewSearchHit':
      return ReviewSearchHit.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ReviewsIdReportPostRequest':
      return ReviewsIdReportPostRequest.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'RevisionPage':
      return RevisionPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Sanction':
      return Sanction.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'SanctionInput':
      return SanctionInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SearchHighlight':
      return SearchHighlight.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SearchHighlightRange':
      return SearchHighlightRange.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SearchResult':
      return SearchResult.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'SearchResultScope':
    case 'SelectionCourse':
      return SelectionCourse.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SelectionOffering':
      return SelectionOffering.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SelectionOfferingPage':
      return SelectionOfferingPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SelectionSyncJob':
      return SelectionSyncJob.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SelectionSyncJobPage':
      return SelectionSyncJobPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Session':
      return Session.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'SessionPage':
      return SessionPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Setting':
      return Setting.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'SettingUpdateInput':
      return SettingUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SigningIntent':
      return SigningIntent.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SigningIntentInput':
      return SigningIntentInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SigningIntentOutcome':
      return SigningIntentOutcome.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SigningIntentOutcomeInput':
      return SigningIntentOutcomeInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'StartupVerifyPostRequest':
      return StartupVerifyPostRequest.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SubmitAppealInput':
      return SubmitAppealInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Subscription':
      return Subscription.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'SubscriptionInput':
      return SubscriptionInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'SubscriptionPage':
      return SubscriptionPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Tag':
      return Tag.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'TagSearchHit':
      return TagSearchHit.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Task':
      return Task.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'TaskAction':
      return TaskAction.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'TaskInput':
      return TaskInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'TaskPage':
      return TaskPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Teacher':
      return Teacher.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Thread':
      return Thread.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ThreadDetail':
      return ThreadDetail.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ThreadDraftPayload':
      return ThreadDraftPayload.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ThreadFeed':
      return ThreadFeed.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ThreadFeedPage':
      return ThreadFeedPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ThreadInput':
      return ThreadInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ThreadMoveInput':
      return ThreadMoveInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ThreadPage':
      return ThreadPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'ThreadPinInput':
      return ThreadPinInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ThreadSearchHit':
      return ThreadSearchHit.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'ThreadUpdateInput':
      return ThreadUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'TimeSlot':
      return TimeSlot.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'TipInput':
      return TipInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'TrustLevelAdjustInput':
      return TrustLevelAdjustInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'TrustLevelEvent':
      return TrustLevelEvent.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'TrustLevelEventPage':
      return TrustLevelEventPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'TrustLevelPolicy':
      return TrustLevelPolicy.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'TrustLevelPolicyPage':
      return TrustLevelPolicyPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'TrustLevelPolicyUpdateInput':
      return TrustLevelPolicyUpdateInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'TrustProgress':
      return TrustProgress.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'UnsanctionInput':
      return UnsanctionInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'UnsubscribeInput':
      return UnsubscribeInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'Upload':
      return Upload.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'UploadCredentials':
      return UploadCredentials.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'UploadIntentInput':
      return UploadIntentInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'UploadPage':
      return UploadPage.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'UserBadge':
      return UserBadge.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'UserComment':
      return UserComment.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'UserCommentPage':
      return UserCommentPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'UserProfile':
      return UserProfile.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'UserProfileWithStats':
      return UserProfileWithStats.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'UserRelationship':
      return UserRelationship.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'UserSearchHit':
      return UserSearchHit.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'UserSummary':
      return UserSummary.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'UserSummaryPage':
      return UserSummaryPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'UserThread':
      return UserThread.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'UserThreadPage':
      return UserThreadPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'VerificationBadgeVariant':
    case 'VerificationCategory':
    case 'VerificationGrant':
      return VerificationGrant.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'VerificationGrantInput':
      return VerificationGrantInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'VerificationGrantPage':
      return VerificationGrantPage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'VerificationIcon':
    case 'VerificationRevokeInput':
      return VerificationRevokeInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'VerificationType':
      return VerificationType.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'VerificationTypeInput':
      return VerificationTypeInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'VerificationTypePage':
      return VerificationTypePage.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'VoteInput':
      return VoteInput.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'VoteResponse':
      return VoteResponse.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'Wallet':
      return Wallet.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'WalletBindPostRequest':
      return WalletBindPostRequest.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'WalletClaimChallenge':
      return WalletClaimChallenge.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'WalletClaimPostRequest':
      return WalletClaimPostRequest.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    case 'WatchedWord':
      return WatchedWord.fromJson(value as Map<String, dynamic>) as ReturnType;
    case 'WatchedWordInput':
      return WatchedWordInput.fromJson(value as Map<String, dynamic>)
          as ReturnType;
    default:
      RegExpMatch? match;

      if (value is List && (match = _regList.firstMatch(targetType)) != null) {
        targetType = match![1]!; // ignore: parameter_assignments
        return value
                .map<BaseType>(
                  (dynamic v) => deserialize<BaseType, BaseType>(
                    v,
                    targetType,
                    growable: growable,
                  ),
                )
                .toList(growable: growable)
            as ReturnType;
      }
      if (value is Set && (match = _regSet.firstMatch(targetType)) != null) {
        targetType = match![1]!; // ignore: parameter_assignments
        return value
                .map<BaseType>(
                  (dynamic v) => deserialize<BaseType, BaseType>(
                    v,
                    targetType,
                    growable: growable,
                  ),
                )
                .toSet()
            as ReturnType;
      }
      if (value is Map && (match = _regMap.firstMatch(targetType)) != null) {
        targetType = match![1]!.trim(); // ignore: parameter_assignments
        return Map<String, BaseType>.fromIterables(
              value.keys as Iterable<String>,
              value.values.map(
                (dynamic v) => deserialize<BaseType, BaseType>(
                  v,
                  targetType,
                  growable: growable,
                ),
              ),
            )
            as ReturnType;
      }
      break;
  }
  throw Exception('Cannot deserialize');
}
