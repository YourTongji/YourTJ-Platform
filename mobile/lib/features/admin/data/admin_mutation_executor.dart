import 'package:dio/dio.dart';
import 'package:uuid/uuid.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../domain/admin_mutations.dart';
import 'admin_repository.dart';

class AdminMutationExecutor {
  const AdminMutationExecutor(this._api);

  final AdminApi _api;

  Future<AdminMutationResult> execute(
    AdminMutationAction action,
    AdminMutationSubmission submission,
    AdminActorContext actor,
  ) async {
    _validateAuthority(action, actor);
    final String reason = submission.reason.trim();
    if (reason.length < 8 || reason.length > 500) {
      throw const AdminMutationValidation('操作理由必须为 8–500 个字符');
    }
    final Map<String, String> values = submission.values;
    if (values.containsKey('selfReviewConfirmed') &&
        !_boolean(values, 'selfReviewConfirmed')) {
      throw const AdminMutationValidation('审核本人媒体必须明确确认管理员例外');
    }
    try {
      switch (action.kind) {
        case AdminMutationKind.inviteUser:
          await _api.adminUsersPost(
            adminUserInviteInput: AdminUserInviteInput(
              email: _required(values, 'email', '校园邮箱'),
              handle: _required(values, 'handle', 'Handle'),
              reason: reason,
            ),
          );
        case AdminMutationKind.changeUserRole:
          await _api.adminUsersIdRolePatch(
            id: _target(action, values),
            adminUserRoleInput: AdminUserRoleInput(
              role: _enumValue(
                AdminUserRoleInputRoleEnum.values,
                _required(values, 'role', '新角色'),
                (AdminUserRoleInputRoleEnum candidate) => candidate.value,
              ),
              reason: reason,
            ),
          );
        case AdminMutationKind.silenceUser:
          await _api.adminUsersIdSilencePost(
            id: _target(action, values),
            sanctionInput: SanctionInput(
              reason: reason,
              endsAt: _requiredInt(values, 'endsAt', '截止时间'),
            ),
          );
        case AdminMutationKind.suspendUser:
          await _api.adminUsersIdSuspendPost(
            id: _target(action, values),
            sanctionInput: SanctionInput(
              reason: reason,
              endsAt: _optionalInt(values, 'endsAt'),
            ),
          );
        case AdminMutationKind.revokeUserSessions:
          await _api.adminUsersIdSessionsRevokePost(
            id: _target(action, values),
            adminReasonInput: AdminReasonInput(reason: reason),
          );
        case AdminMutationKind.revokeSilence ||
            AdminMutationKind.revokeSuspension:
          await _api.adminUsersIdUnsanctionPost(
            id: _target(action, values),
            unsanctionInput: UnsanctionInput(
              sanctionId: _required(values, 'sanctionId', '制裁记录 ID'),
              reason: reason,
            ),
          );
        case AdminMutationKind.adjustTrustLevel:
          final bool clearOverride = _boolean(values, 'clearOverride');
          await _api.adminUsersIdTrustLevelPatch(
            id: _target(action, values),
            trustLevelAdjustInput: TrustLevelAdjustInput(
              trustLevel: clearOverride
                  ? null
                  : _requiredInt(values, 'trustLevel', '信任等级'),
              clearOverride: clearOverride,
              reason: reason,
            ),
          );
        case AdminMutationKind.toggleReview:
          await _api.adminReviewsIdTogglePost(
            id: _target(action, values),
            adminReasonInput: AdminReasonInput(reason: reason),
          );
        case AdminMutationKind.deleteReview:
          await _api.adminReviewsIdDelete(
            id: _target(action, values),
            adminReasonInput: AdminReasonInput(reason: reason),
          );
        case AdminMutationKind.resolveReviewReport:
          await _api.adminReportsIdResolvePost(
            id: _target(action, values),
            reviewReportResolutionInput: ReviewReportResolutionInput(
              action: _enumValue(
                ReviewReportResolutionInputActionEnum.values,
                _required(values, 'action', '举报决定'),
                (ReviewReportResolutionInputActionEnum candidate) =>
                    candidate.value,
              ),
              note: reason,
            ),
          );
        case AdminMutationKind.resolveForumFlag:
          await _api.adminForumFlagsIdResolvePost(
            id: _target(action, values),
            flagResolveInput: FlagResolveInput(
              action: _enumValue(
                FlagResolveInputActionEnum.values,
                _required(values, 'action', '举报决定'),
                (FlagResolveInputActionEnum candidate) => candidate.value,
              ),
              note: reason,
            ),
          );
        case AdminMutationKind.resolveDirectMessageReport:
          await _api.adminDmReportsIdResolvePost(
            id: _target(action, values),
            dmReportResolutionInput: DmReportResolutionInput(
              action: _enumValue(
                DmReportResolutionInputActionEnum.values,
                _required(values, 'action', '举报决定'),
                (DmReportResolutionInputActionEnum candidate) =>
                    candidate.value,
              ),
              note: reason,
            ),
          );
        case AdminMutationKind.moderateForumThread:
          await _api.adminForumThreadsIdActionPost(
            id: _target(action, values),
            action: _required(values, 'action', '治理动作'),
            adminThreadActionInput: AdminThreadActionInput(
              reason: reason,
              globally: _boolean(values, 'globally'),
              boardId: _optional(values, 'boardId'),
            ),
          );
        case AdminMutationKind.moderateForumComment:
          await _api.adminForumCommentsIdActionPost(
            id: _target(action, values),
            action: _required(values, 'action', '治理动作'),
            adminCommentActionInput: AdminCommentActionInput(reason: reason),
          );
        case AdminMutationKind.featureForumThread:
          await _api.adminForumThreadsIdFeaturePost(
            id: _target(action, values),
            featureThreadInput: FeatureThreadInput(
              featured: _boolean(values, 'featured'),
              reason: reason,
            ),
          );
        case AdminMutationKind.startAppealReview:
          await _api.adminAppealsIdReviewPost(
            id: _target(action, values),
            appealTransitionInput: AppealTransitionInput(
              expectedVersion: _expectedVersion(action),
              reason: reason,
            ),
          );
        case AdminMutationKind.decideAppeal:
          await _api.adminAppealsIdDecisionPost(
            id: _target(action, values),
            appealDecisionInput: AppealDecisionInput(
              expectedVersion: _expectedVersion(action),
              outcome: _enumValue(
                AppealDecisionInputOutcomeEnum.values,
                _required(values, 'outcome', '申诉结果'),
                (AppealDecisionInputOutcomeEnum candidate) => candidate.value,
              ),
              reason: reason,
              amendedEndsAt: _optionalInt(values, 'amendedEndsAt'),
            ),
          );
        case AdminMutationKind.previewMedia:
          final Response<ModerationPreviewGrant> grantResponse = await _api
              .adminMediaUploadsIdPreviewGrantsPost(
                id: _target(action, values),
                mediaModerationInput: MediaModerationInput(
                  reason: reason,
                  selfReviewConfirmed: _boolean(values, 'selfReviewConfirmed'),
                ),
              );
          final ModerationPreviewGrant grant = _responseData(
            grantResponse,
            '媒体预览授权',
          );
          final response = await _api.adminMediaUploadsIdPreviewGet(
            id: _target(action, values),
            xMediaPreviewToken: grant.token,
          );
          return AdminMutationResult.preview(_responseData(response, '媒体安全预览'));
        case AdminMutationKind.approveMedia:
          await _api.adminMediaUploadsIdApprovePost(
            id: _target(action, values),
            mediaModerationInput: MediaModerationInput(
              reason: reason,
              selfReviewConfirmed: _boolean(values, 'selfReviewConfirmed'),
            ),
          );
        case AdminMutationKind.blockMedia:
          await _api.adminMediaUploadsIdBlockPost(
            id: _target(action, values),
            mediaModerationInput: MediaModerationInput(
              reason: reason,
              selfReviewConfirmed: _boolean(values, 'selfReviewConfirmed'),
            ),
          );
        case AdminMutationKind.retryMediaProcessing:
          await _api.adminMediaUploadsIdProcessingRetryPost(
            id: _target(action, values),
            adminReasonInput: AdminReasonInput(reason: reason),
          );
        case AdminMutationKind.placeMediaRetentionHold:
          await _api.adminMediaUploadsIdRetentionHoldPost(
            id: _target(action, values),
            mediaRetentionHoldInput: MediaRetentionHoldInput(
              holdKind: _enumValue(
                MediaRetentionHoldInputHoldKindEnum.values,
                _required(values, 'holdKind', '保留类型'),
                (MediaRetentionHoldInputHoldKindEnum candidate) =>
                    candidate.value,
              ),
              expiresAt: _requiredInt(values, 'expiresAt', '到期时间'),
              reason: reason,
              expectedHoldId: _optional(values, 'expectedHoldId'),
            ),
          );
        case AdminMutationKind.releaseMediaRetentionHold:
          await _api.adminMediaUploadsIdRetentionHoldDelete(
            id: _target(action, values),
            mediaRetentionHoldReleaseInput: MediaRetentionHoldReleaseInput(
              expectedHoldId: _required(values, 'expectedHoldId', '已审阅保留 ID'),
              reason: reason,
            ),
          );
        case AdminMutationKind.retryMediaDeletion:
          await _api.adminMediaDeletionJobsIdRetryPost(
            id: _target(action, values),
            adminReasonInput: AdminReasonInput(reason: reason),
          );
        default:
          return await _executeConfiguration(action, submission, actor);
      }
      return const AdminMutationResult.success();
    } on DioException catch (error) {
      if (error.response?.statusCode == 428 ||
          _errorCode(error.response?.data) == 'RECENT_AUTH_REQUIRED') {
        throw const AdminRecentAuthenticationRequired();
      }
      throw ApiFailure.fromDio(error);
    }
  }

  Future<AdminMutationResult> _executeConfiguration(
    AdminMutationAction action,
    AdminMutationSubmission submission,
    AdminActorContext actor,
  ) async {
    final Map<String, String> values = submission.values;
    final String reason = submission.reason.trim();
    switch (action.kind) {
      case AdminMutationKind.createCourse:
        await _api.adminCoursesPost(
          adminCourseCreateInput: AdminCourseCreateInput(
            code: _required(values, 'code', '课程代码'),
            name: _required(values, 'name', '课程名称'),
            credit: _optionalNumber(values, 'credit'),
            department: _optional(values, 'department'),
            teacherName: _optional(values, 'teacherName'),
            reason: reason,
          ),
        );
      case AdminMutationKind.updateCourse:
        await _api.adminCoursesIdPut(
          id: _target(action, values),
          adminCourseUpdateInput: AdminCourseUpdateInput(
            code: _optional(values, 'code'),
            name: _optional(values, 'name'),
            credit: _optionalNumber(values, 'credit'),
            department: _optional(values, 'department'),
            teacherName: _optional(values, 'teacherName'),
            reason: reason,
          ),
        );
      case AdminMutationKind.deleteCourse:
        await _api.adminCoursesIdDelete(
          id: _target(action, values),
          adminReasonInput: AdminReasonInput(reason: reason),
        );
      case AdminMutationKind.createBoard:
        await _api.adminForumBoardsPost(
          adminBoardCreateInput: AdminBoardCreateInput(
            slug: _required(values, 'slug', 'Slug'),
            name: _required(values, 'name', '板块名称'),
            description: _optional(values, 'description'),
            position: _optionalInt(values, 'position'),
            isLocked: _boolean(values, 'isLocked'),
            minTrustToPost: _optionalInt(values, 'minTrustToPost'),
            isQa: _boolean(values, 'isQa'),
            reason: reason,
          ),
        );
      case AdminMutationKind.updateBoard:
        await _api.adminForumBoardsIdPatch(
          id: _target(action, values),
          adminBoardUpdateInput: AdminBoardUpdateInput(
            slug: _optional(values, 'slug'),
            name: _optional(values, 'name'),
            description: _optional(values, 'description'),
            position: _optionalInt(values, 'position'),
            isLocked: _optionalBoolean(values, 'isLocked'),
            minTrustToPost: _optionalInt(values, 'minTrustToPost'),
            isQa: _optionalBoolean(values, 'isQa'),
            reason: reason,
          ),
        );
      case AdminMutationKind.deleteBoard:
        await _api.adminForumBoardsIdDelete(
          id: _target(action, values),
          adminReasonInput: AdminReasonInput(reason: reason),
        );
      case AdminMutationKind.createTag:
        await _api.adminForumTagsPost(
          adminTagCreateInput: AdminTagCreateInput(
            slug: _required(values, 'slug', 'Slug'),
            name: _required(values, 'name', '标签名称'),
            description: _optional(values, 'description'),
            reason: reason,
          ),
        );
      case AdminMutationKind.updateTag:
        await _api.adminForumTagsIdPatch(
          id: _target(action, values),
          adminTagUpdateInput: AdminTagUpdateInput(
            slug: _optional(values, 'slug'),
            name: _optional(values, 'name'),
            description: _optional(values, 'description'),
            reason: reason,
          ),
        );
      case AdminMutationKind.deleteTag:
        await _api.adminForumTagsIdDelete(
          id: _target(action, values),
          adminReasonInput: AdminReasonInput(reason: reason),
        );
      case AdminMutationKind.createWatchedWord:
        await _api.adminForumWatchedWordsPost(
          watchedWordInput: WatchedWordInput(
            word: _required(values, 'word', '关注词'),
            action: _enumValue(
              WatchedWordInputActionEnum.values,
              _required(values, 'action', '关注词动作'),
              (WatchedWordInputActionEnum candidate) => candidate.value,
            ),
            reason: reason,
          ),
        );
      case AdminMutationKind.deleteWatchedWord:
        await _api.adminForumWatchedWordsIdDelete(
          id: _target(action, values),
          adminReasonInput: AdminReasonInput(reason: reason),
        );
      case AdminMutationKind.updateActivityPolicy:
        await _api.adminActivityPolicyPut(
          activityPolicyUpdateInput: ActivityPolicyUpdateInput(
            expectedVersion: _expectedVersion(action),
            weights: ActivityWeights(
              thread: _requiredInt(values, 'thread', '主题权重'),
              comment: _requiredInt(values, 'comment', '评论权重'),
              like: _requiredInt(values, 'like', '点赞权重'),
              checkIn: _requiredInt(values, 'checkIn', '签到权重'),
            ),
            reason: reason,
          ),
        );
      case AdminMutationKind.updateTrustPolicy:
        await _api.adminTrustPolicyPut(
          trustLevelPolicyUpdateInput: TrustLevelPolicyUpdateInput(
            expectedVersion: _expectedVersion(action),
            thresholdLevel2: _requiredInt(values, 'thresholdLevel2', 'Lv.2 阈值'),
            thresholdLevel3: _requiredInt(values, 'thresholdLevel3', 'Lv.3 阈值'),
            thresholdLevel4: _requiredInt(values, 'thresholdLevel4', 'Lv.4 阈值'),
            thresholdLevel5: _requiredInt(values, 'thresholdLevel5', 'Lv.5 阈值'),
            thresholdLevel6: _requiredInt(values, 'thresholdLevel6', 'Lv.6 阈值'),
            likeDailyCap: _requiredInt(values, 'likeDailyCap', '点赞日上限'),
            demotionCooldownDays: _requiredInt(
              values,
              'demotionCooldownDays',
              '降级冷却天数',
            ),
            reason: reason,
          ),
        );
      case AdminMutationKind.createAnnouncement:
        await _api.adminAnnouncementsPost(
          announcementCreateInput: AnnouncementCreateInput(
            title: _required(values, 'title', '公告标题'),
            body: _optional(values, 'body'),
            status: _enumValue(
              AnnouncementCreateInputStatusEnum.values,
              _required(values, 'status', '公告状态'),
              (AnnouncementCreateInputStatusEnum candidate) => candidate.value,
            ),
            presentation: _enumValue(
              AnnouncementCreateInputPresentationEnum.values,
              _required(values, 'presentation', '展示方式'),
              (AnnouncementCreateInputPresentationEnum candidate) =>
                  candidate.value,
            ),
            severity: _enumValue(
              AnnouncementCreateInputSeverityEnum.values,
              _required(values, 'severity', '公告级别'),
              (AnnouncementCreateInputSeverityEnum candidate) =>
                  candidate.value,
            ),
            priority: _requiredInt(values, 'priority', '优先级'),
            audience: _enumValue(
              AnnouncementCreateInputAudienceEnum.values,
              _required(values, 'audience', '公告受众'),
              (AnnouncementCreateInputAudienceEnum candidate) =>
                  candidate.value,
            ),
            requiresAck: _boolean(values, 'requiresAck'),
            startsAt: _optionalInt(values, 'startsAt'),
            endsAt: _optionalInt(values, 'endsAt'),
            reason: reason,
          ),
        );
      case AdminMutationKind.updateAnnouncement:
        await _api.adminAnnouncementsIdPatch(
          id: _target(action, values),
          announcementUpdateInput: AnnouncementUpdateInput(
            title: _required(values, 'title', '公告标题'),
            body: _optional(values, 'body'),
            status: _enumValue(
              AnnouncementUpdateInputStatusEnum.values,
              _required(values, 'status', '公告状态'),
              (AnnouncementUpdateInputStatusEnum candidate) => candidate.value,
            ),
            presentation: _enumValue(
              AnnouncementUpdateInputPresentationEnum.values,
              _required(values, 'presentation', '展示方式'),
              (AnnouncementUpdateInputPresentationEnum candidate) =>
                  candidate.value,
            ),
            severity: _enumValue(
              AnnouncementUpdateInputSeverityEnum.values,
              _required(values, 'severity', '公告级别'),
              (AnnouncementUpdateInputSeverityEnum candidate) =>
                  candidate.value,
            ),
            priority: _requiredInt(values, 'priority', '优先级'),
            audience: _enumValue(
              AnnouncementUpdateInputAudienceEnum.values,
              _required(values, 'audience', '公告受众'),
              (AnnouncementUpdateInputAudienceEnum candidate) =>
                  candidate.value,
            ),
            requiresAck: _boolean(values, 'requiresAck'),
            startsAt: _optionalInt(values, 'startsAt'),
            endsAt: _optionalInt(values, 'endsAt'),
            reason: reason,
            expectedVersion: _expectedVersion(action),
            bumpRevision: _boolean(values, 'bumpRevision'),
          ),
        );
      case AdminMutationKind.archiveAnnouncement:
        await _api.adminAnnouncementsIdDelete(
          id: _target(action, values),
          adminVersionedArchiveInput: AdminVersionedArchiveInput(
            expectedVersion: _expectedVersion(action),
            reason: reason,
          ),
        );
      case AdminMutationKind.createPromotion:
        await _api.adminPromotionsPost(
          promotionCreateInput: PromotionCreateInput(
            placement: _enumValue(
              PromotionCreateInputPlacementEnum.values,
              _required(values, 'placement', '推广位置'),
              (PromotionCreateInputPlacementEnum candidate) => candidate.value,
            ),
            title: _required(values, 'title', '推广标题'),
            body: _optional(values, 'body'),
            ctaLabel: _optional(values, 'ctaLabel'),
            targetUrl: _required(values, 'targetUrl', '目标 URL'),
            assetId: _optional(values, 'assetId'),
            status: _enumValue(
              PromotionCreateInputStatusEnum.values,
              _required(values, 'status', '推广状态'),
              (PromotionCreateInputStatusEnum candidate) => candidate.value,
            ),
            priority: _requiredInt(values, 'priority', '优先级'),
            audience: _enumValue(
              PromotionCreateInputAudienceEnum.values,
              _required(values, 'audience', '推广受众'),
              (PromotionCreateInputAudienceEnum candidate) => candidate.value,
            ),
            startsAt: _optionalInt(values, 'startsAt'),
            endsAt: _optionalInt(values, 'endsAt'),
            reason: reason,
          ),
        );
      case AdminMutationKind.updatePromotion:
        await _api.adminPromotionsIdPatch(
          id: _target(action, values),
          promotionUpdateInput: PromotionUpdateInput(
            placement: _enumValue(
              PromotionUpdateInputPlacementEnum.values,
              _required(values, 'placement', '推广位置'),
              (PromotionUpdateInputPlacementEnum candidate) => candidate.value,
            ),
            title: _required(values, 'title', '推广标题'),
            body: _optional(values, 'body'),
            ctaLabel: _optional(values, 'ctaLabel'),
            targetUrl: _required(values, 'targetUrl', '目标 URL'),
            assetId: _optional(values, 'assetId'),
            status: _enumValue(
              PromotionUpdateInputStatusEnum.values,
              _required(values, 'status', '推广状态'),
              (PromotionUpdateInputStatusEnum candidate) => candidate.value,
            ),
            priority: _requiredInt(values, 'priority', '优先级'),
            audience: _enumValue(
              PromotionUpdateInputAudienceEnum.values,
              _required(values, 'audience', '推广受众'),
              (PromotionUpdateInputAudienceEnum candidate) => candidate.value,
            ),
            startsAt: _optionalInt(values, 'startsAt'),
            endsAt: _optionalInt(values, 'endsAt'),
            reason: reason,
            expectedVersion: _expectedVersion(action),
          ),
        );
      case AdminMutationKind.archivePromotion:
        await _api.adminPromotionsIdDelete(
          id: _target(action, values),
          adminVersionedArchiveInput: AdminVersionedArchiveInput(
            expectedVersion: _expectedVersion(action),
            reason: reason,
          ),
        );
      default:
        return await _executeGovernanceDefinitions(action, submission, actor);
    }
    return const AdminMutationResult.success();
  }

  Future<AdminMutationResult> _executeGovernanceDefinitions(
    AdminMutationAction action,
    AdminMutationSubmission submission,
    AdminActorContext actor,
  ) async {
    final Map<String, String> values = submission.values;
    final String reason = submission.reason.trim();
    switch (action.kind) {
      case AdminMutationKind.createAchievement:
        await _api.adminAchievementsPost(
          achievementCreateInput: AchievementCreateInput(
            slug: _required(values, 'slug', '成就 Slug'),
            name: _required(values, 'name', '成就名称'),
            description: _optional(values, 'description'),
            icon: _enumValue(
              AchievementIcon.values,
              _required(values, 'icon', '成就图标'),
              (AchievementIcon candidate) => candidate.value,
            ),
            mintAmount: _requiredInt(values, 'mintAmount', '自动规则积分'),
            reason: reason,
          ),
        );
      case AdminMutationKind.updateAchievement:
        await _api.adminAchievementsAchievementIdPatch(
          achievementId: _target(action, values),
          achievementUpdateInput: AchievementUpdateInput(
            expectedVersion: _expectedVersion(action),
            name: _required(values, 'name', '成就名称'),
            description: _optional(values, 'description'),
            icon: _enumValue(
              AchievementIcon.values,
              _required(values, 'icon', '成就图标'),
              (AchievementIcon candidate) => candidate.value,
            ),
            status: _enumValue(
              AchievementStatus.values,
              _required(values, 'status', '成就状态'),
              (AchievementStatus candidate) => candidate.value,
            ),
            mintAmount: _requiredInt(values, 'mintAmount', '自动规则积分'),
            reason: reason,
          ),
        );
      case AdminMutationKind.grantAchievement:
        await _api.adminUsersIdAchievementsPost(
          id: _required(values, 'accountId', '账号 ID'),
          achievementGrantInput: AchievementGrantInput(
            achievementId: _required(values, 'achievementId', '成就 ID'),
            reason: reason,
          ),
        );
      case AdminMutationKind.revokeAchievement:
        await _api.adminUsersIdAchievementsAchievementIdRevokePost(
          id: _required(values, 'accountId', '账号 ID'),
          achievementId: _required(values, 'achievementId', '成就 ID'),
          achievementRevokeInput: AchievementRevokeInput(reason: reason),
        );
      case AdminMutationKind.createVerificationType:
        await _api.adminVerificationsTypesPost(
          verificationTypeInput: VerificationTypeInput(
            slug: _required(values, 'slug', '认证 Slug'),
            category: _enumValue(
              VerificationCategory.values,
              _required(values, 'category', '认证类别'),
              (VerificationCategory candidate) => candidate.value,
            ),
            label: _required(values, 'label', '认证名称'),
            description: _optional(values, 'description'),
            icon: _enumValue(
              VerificationIcon.values,
              _required(values, 'icon', '认证图标'),
              (VerificationIcon candidate) => candidate.value,
            ),
            badgeVariant: _enumValue(
              VerificationBadgeVariant.values,
              _required(values, 'badgeVariant', '徽标样式'),
              (VerificationBadgeVariant candidate) => candidate.value,
            ),
            allowsPublicDisplay: _boolean(values, 'allowsPublicDisplay'),
            reason: reason,
          ),
        );
      case AdminMutationKind.grantVerification:
        await _api.adminUsersIdVerificationsPost(
          id: _required(values, 'accountId', '账号 ID'),
          verificationGrantInput: VerificationGrantInput(
            verificationTypeId: _required(
              values,
              'verificationTypeId',
              '认证类型 ID',
            ),
            displayOnProfile: _boolean(values, 'displayOnProfile'),
            expiresAt: _optionalInt(values, 'expiresAt'),
            evidenceReference: _optional(values, 'evidenceReference'),
            reason: reason,
          ),
        );
      case AdminMutationKind.revokeVerification:
        await _api.adminVerificationsGrantsIdRevokePost(
          id: _required(values, 'grantId', '认证授予 ID'),
          verificationRevokeInput: VerificationRevokeInput(reason: reason),
        );
      case AdminMutationKind.startCreditReconciliation:
        await _api.adminCreditReconciliationsPost(
          idempotencyKey: const Uuid().v4(),
          reconciliationRunInput: ReconciliationRunInput(reason: reason),
        );
      case AdminMutationKind.resumeCreditReconciliation:
        await _api.adminCreditReconciliationsIdResumePost(
          id: _target(action, values),
          reconciliationRunInput: ReconciliationRunInput(reason: reason),
        );
      case AdminMutationKind.updateSetting:
        await _api.adminSettingsKeyPut(
          key: _target(action, values),
          settingUpdateInput: SettingUpdateInput(
            value: _required(values, 'value', '设置值'),
            reason: reason,
          ),
        );
      case AdminMutationKind.triggerSelectionSync:
        await _api.adminSelectionSyncPost(
          idempotencyKey: const Uuid().v4(),
          adminReasonInput: AdminReasonInput(reason: reason),
        );
      case AdminMutationKind.reindexCourses:
        await _api.adminCoursesReindexPost(
          adminReasonInput: AdminReasonInput(reason: reason),
        );
      case AdminMutationKind.reindexReviews:
        await _api.adminReviewsReindexPost(
          adminReasonInput: AdminReasonInput(reason: reason),
        );
      case AdminMutationKind.reindexForum:
        await _api.adminForumReindexPost(
          adminReasonInput: AdminReasonInput(reason: reason),
        );
      case AdminMutationKind.retryNotificationOutbox:
        await _api.adminNotificationOutboxIdRetryPost(
          id: _target(action, values),
          notificationOutboxRetryInput: NotificationOutboxRetryInput(
            reason: reason,
          ),
        );
      case AdminMutationKind.requeueLifecycleJob:
        await _api.adminAccountLifecycleJobsIdRequeuePost(
          id: _target(action, values),
          adminReasonInput: AdminReasonInput(reason: reason),
        );
      case AdminMutationKind.inviteUser ||
          AdminMutationKind.changeUserRole ||
          AdminMutationKind.silenceUser ||
          AdminMutationKind.suspendUser ||
          AdminMutationKind.revokeUserSessions ||
          AdminMutationKind.revokeSilence ||
          AdminMutationKind.revokeSuspension ||
          AdminMutationKind.adjustTrustLevel ||
          AdminMutationKind.toggleReview ||
          AdminMutationKind.deleteReview ||
          AdminMutationKind.resolveReviewReport ||
          AdminMutationKind.resolveForumFlag ||
          AdminMutationKind.resolveDirectMessageReport ||
          AdminMutationKind.moderateForumThread ||
          AdminMutationKind.moderateForumComment ||
          AdminMutationKind.featureForumThread ||
          AdminMutationKind.startAppealReview ||
          AdminMutationKind.decideAppeal ||
          AdminMutationKind.previewMedia ||
          AdminMutationKind.approveMedia ||
          AdminMutationKind.blockMedia ||
          AdminMutationKind.retryMediaProcessing ||
          AdminMutationKind.placeMediaRetentionHold ||
          AdminMutationKind.releaseMediaRetentionHold ||
          AdminMutationKind.retryMediaDeletion ||
          AdminMutationKind.createCourse ||
          AdminMutationKind.updateCourse ||
          AdminMutationKind.deleteCourse ||
          AdminMutationKind.createBoard ||
          AdminMutationKind.updateBoard ||
          AdminMutationKind.deleteBoard ||
          AdminMutationKind.createTag ||
          AdminMutationKind.updateTag ||
          AdminMutationKind.deleteTag ||
          AdminMutationKind.createWatchedWord ||
          AdminMutationKind.deleteWatchedWord ||
          AdminMutationKind.updateActivityPolicy ||
          AdminMutationKind.updateTrustPolicy ||
          AdminMutationKind.createAnnouncement ||
          AdminMutationKind.updateAnnouncement ||
          AdminMutationKind.archiveAnnouncement ||
          AdminMutationKind.createPromotion ||
          AdminMutationKind.updatePromotion ||
          AdminMutationKind.archivePromotion:
        throw const AdminMutationValidation('管理操作内部分派不一致，已阻止提交');
    }
    return const AdminMutationResult.success();
  }

  static void _validateAuthority(
    AdminMutationAction action,
    AdminActorContext actor,
  ) {
    if (!action.requiredAnyCapability.any(actor.capabilities.contains)) {
      throw const AdminAccessDenied();
    }
    final String? targetAccountId = action.targetAccountId;
    final String? targetRole = action.targetRole;
    if ((targetAccountId == null) != (targetRole == null)) {
      throw const AdminMutationValidation('用户治理目标缺少完整层级证据');
    }
    if (targetAccountId != null &&
        targetRole != null &&
        !actor.canManageTarget(accountId: targetAccountId, role: targetRole)) {
      throw const AdminAccessDenied();
    }
  }

  static String _target(
    AdminMutationAction action,
    Map<String, String> values,
  ) => action.targetId ?? _required(values, 'targetId', '目标 ID');

  static int _expectedVersion(AdminMutationAction action) {
    final int? version = action.expectedVersion;
    if (version == null) {
      throw const AdminMutationValidation('操作缺少已审阅版本，不能提交');
    }
    return version;
  }

  static String _required(
    Map<String, String> values,
    String key,
    String label,
  ) {
    final String value = values[key]?.trim() ?? '';
    if (value.isEmpty) {
      throw AdminMutationValidation('$label 不能为空');
    }
    return value;
  }

  static String? _optional(Map<String, String> values, String key) {
    final String value = values[key]?.trim() ?? '';
    return value.isEmpty ? null : value;
  }

  static int _requiredInt(
    Map<String, String> values,
    String key,
    String label,
  ) {
    final int? parsed = int.tryParse(_required(values, key, label));
    if (parsed == null) {
      throw AdminMutationValidation('$label 必须是整数');
    }
    return parsed;
  }

  static int? _optionalInt(Map<String, String> values, String key) {
    final String? value = _optional(values, key);
    if (value == null) {
      return null;
    }
    final int? parsed = int.tryParse(value);
    if (parsed == null) {
      throw AdminMutationValidation('$key 必须是整数');
    }
    return parsed;
  }

  static num? _optionalNumber(Map<String, String> values, String key) {
    final String? value = _optional(values, key);
    if (value == null) {
      return null;
    }
    final num? parsed = num.tryParse(value);
    if (parsed == null) {
      throw AdminMutationValidation('$key 必须是数字');
    }
    return parsed;
  }

  static bool _boolean(Map<String, String> values, String key) =>
      values[key] == 'true';

  static bool? _optionalBoolean(Map<String, String> values, String key) {
    final String? value = _optional(values, key);
    if (value == null) {
      return null;
    }
    if (value != 'true' && value != 'false') {
      throw AdminMutationValidation('$key 必须选择保持、是或否');
    }
    return value == 'true';
  }

  static T _enumValue<T>(
    List<T> values,
    String wireValue,
    String Function(T candidate) readValue,
  ) {
    for (final T candidate in values) {
      if (readValue(candidate) == wireValue) {
        return candidate;
      }
    }
    throw AdminMutationValidation('不支持的枚举值：$wireValue');
  }

  static T _responseData<T>(Response<T> response, String surface) {
    final T? data = response.data;
    if (data == null) {
      throw AdminMutationValidation('$surface 返回空响应');
    }
    return data;
  }

  static String? _errorCode(Object? data) {
    if (data is! Map) {
      return null;
    }
    final Object? error = data['error'];
    if (error is! Map) {
      return null;
    }
    final Object? code = error['code'];
    return code is String ? code : null;
  }
}
