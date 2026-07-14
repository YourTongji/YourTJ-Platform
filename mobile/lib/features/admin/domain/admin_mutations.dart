import 'dart:typed_data';

enum AdminMutationKind {
  inviteUser,
  changeUserRole,
  silenceUser,
  suspendUser,
  revokeUserSessions,
  revokeSilence,
  revokeSuspension,
  adjustTrustLevel,
  toggleReview,
  deleteReview,
  resolveReviewReport,
  resolveForumFlag,
  resolveDirectMessageReport,
  moderateForumThread,
  moderateForumComment,
  featureForumThread,
  startAppealReview,
  decideAppeal,
  previewMedia,
  approveMedia,
  blockMedia,
  retryMediaProcessing,
  placeMediaRetentionHold,
  releaseMediaRetentionHold,
  retryMediaDeletion,
  createCourse,
  updateCourse,
  deleteCourse,
  createBoard,
  updateBoard,
  deleteBoard,
  createTag,
  updateTag,
  deleteTag,
  createWatchedWord,
  deleteWatchedWord,
  updateActivityPolicy,
  updateTrustPolicy,
  createAnnouncement,
  updateAnnouncement,
  archiveAnnouncement,
  createPromotion,
  updatePromotion,
  archivePromotion,
  createAchievement,
  updateAchievement,
  grantAchievement,
  revokeAchievement,
  createVerificationType,
  grantVerification,
  revokeVerification,
  startCreditReconciliation,
  resumeCreditReconciliation,
  updateSetting,
  triggerSelectionSync,
  reindexCourses,
  reindexReviews,
  reindexForum,
  retryNotificationOutbox,
  requeueLifecycleJob,
}

enum AdminMutationFieldKind {
  text,
  multiline,
  integer,
  decimal,
  boolean,
  choice,
}

class AdminMutationOption {
  const AdminMutationOption(this.value, this.label);

  final String value;
  final String label;
}

class AdminMutationField {
  const AdminMutationField({
    required this.key,
    required this.label,
    this.kind = AdminMutationFieldKind.text,
    this.initialValue = '',
    this.isRequired = false,
    this.mustBeTrue = false,
    this.helperText,
    this.options = const <AdminMutationOption>[],
  }) : assert(!mustBeTrue || kind == AdminMutationFieldKind.boolean);

  final String key;
  final String label;
  final AdminMutationFieldKind kind;
  final String initialValue;
  final bool isRequired;
  final bool mustBeTrue;
  final String? helperText;
  final List<AdminMutationOption> options;
}

class AdminMutationAction {
  const AdminMutationAction({
    required this.kind,
    required this.label,
    required this.impact,
    required this.requiredAnyCapability,
    this.targetId,
    this.targetAccountId,
    this.targetRole,
    this.expectedVersion,
    this.isDestructive = false,
    this.requiresRecentAuth = true,
    this.fields = const <AdminMutationField>[],
  });

  final AdminMutationKind kind;
  final String label;
  final String impact;
  final Set<String> requiredAnyCapability;
  final String? targetId;
  final String? targetAccountId;
  final String? targetRole;
  final int? expectedVersion;
  final bool isDestructive;
  final bool requiresRecentAuth;
  final List<AdminMutationField> fields;
}

class AdminMutationSubmission {
  const AdminMutationSubmission({required this.reason, required this.values});

  final String reason;
  final Map<String, String> values;
}

class AdminMutationResult {
  const AdminMutationResult.success({this.message = '管理操作已完成'})
    : previewBytes = null;

  const AdminMutationResult.preview(Uint8List this.previewBytes)
    : message = '一次性安全预览已加载';

  final String message;
  final Uint8List? previewBytes;
}

class AdminMutationValidation implements Exception {
  const AdminMutationValidation(this.message);

  final String message;

  @override
  String toString() => message;
}
