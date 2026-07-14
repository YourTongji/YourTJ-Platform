// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'upload.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Upload _$UploadFromJson(Map<String, dynamic> json) => $checkedCreate(
  'Upload',
  json,
  ($checkedConvert) {
    $checkKeys(
      json,
      requiredKeys: const [
        'id',
        'accountId',
        'kind',
        'bytes',
        'mime',
        'status',
        'deliveryState',
        'deliveryErrorCode',
        'usage',
        'imageWidth',
        'imageHeight',
        'isSelfReview',
        'approvalRequirement',
        'deletionState',
        'retentionHeld',
        'retentionState',
        'retentionExpiresAt',
        'createdAt',
      ],
    );
    final val = Upload(
      id: $checkedConvert('id', (v) => v as String),
      accountId: $checkedConvert('accountId', (v) => v as String),
      kind: $checkedConvert(
        'kind',
        (v) => $enumDecode(
          _$UploadKindEnumEnumMap,
          v,
          unknownValue: UploadKindEnum.unknownDefaultOpenApi,
        ),
      ),
      bytes: $checkedConvert('bytes', (v) => (v as num).toInt()),
      mime: $checkedConvert('mime', (v) => v as String),
      status: $checkedConvert(
        'status',
        (v) => $enumDecode(
          _$UploadStatusEnumEnumMap,
          v,
          unknownValue: UploadStatusEnum.unknownDefaultOpenApi,
        ),
      ),
      deliveryState: $checkedConvert(
        'deliveryState',
        (v) => $enumDecode(
          _$MediaDeliveryStateEnumMap,
          v,
          unknownValue: MediaDeliveryState.unknownDefaultOpenApi,
        ),
      ),
      deliveryErrorCode: $checkedConvert(
        'deliveryErrorCode',
        (v) => $enumDecodeNullable(
          _$UploadDeliveryErrorCodeEnumEnumMap,
          v,
          unknownValue: UploadDeliveryErrorCodeEnum.unknownDefaultOpenApi,
        ),
      ),
      usage: $checkedConvert(
        'usage',
        (v) => $enumDecodeNullable(
          _$MediaUsageEnumMap,
          v,
          unknownValue: MediaUsage.unknownDefaultOpenApi,
        ),
      ),
      imageWidth: $checkedConvert('imageWidth', (v) => (v as num?)?.toInt()),
      imageHeight: $checkedConvert('imageHeight', (v) => (v as num?)?.toInt()),
      isSelfReview: $checkedConvert('isSelfReview', (v) => v as bool),
      approvalRequirement: $checkedConvert(
        'approvalRequirement',
        (v) => $enumDecode(
          _$UploadApprovalRequirementEnumEnumMap,
          v,
          unknownValue: UploadApprovalRequirementEnum.unknownDefaultOpenApi,
        ),
      ),
      deletionState: $checkedConvert(
        'deletionState',
        (v) => $enumDecodeNullable(
          _$UploadDeletionStateEnumEnumMap,
          v,
          unknownValue: UploadDeletionStateEnum.unknownDefaultOpenApi,
        ),
      ),
      retentionHeld: $checkedConvert('retentionHeld', (v) => v as bool),
      retentionState: $checkedConvert(
        'retentionState',
        (v) => $enumDecode(
          _$UploadRetentionStateEnumEnumMap,
          v,
          unknownValue: UploadRetentionStateEnum.unknownDefaultOpenApi,
        ),
      ),
      retentionExpiresAt: $checkedConvert(
        'retentionExpiresAt',
        (v) => (v as num?)?.toInt(),
      ),
      createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
    );
    return val;
  },
);

Map<String, dynamic> _$UploadToJson(Upload instance) => <String, dynamic>{
  'id': instance.id,
  'accountId': instance.accountId,
  'kind': _$UploadKindEnumEnumMap[instance.kind]!,
  'bytes': instance.bytes,
  'mime': instance.mime,
  'status': _$UploadStatusEnumEnumMap[instance.status]!,
  'deliveryState': _$MediaDeliveryStateEnumMap[instance.deliveryState]!,
  'deliveryErrorCode':
      _$UploadDeliveryErrorCodeEnumEnumMap[instance.deliveryErrorCode],
  'usage': _$MediaUsageEnumMap[instance.usage],
  'imageWidth': instance.imageWidth,
  'imageHeight': instance.imageHeight,
  'isSelfReview': instance.isSelfReview,
  'approvalRequirement':
      _$UploadApprovalRequirementEnumEnumMap[instance.approvalRequirement]!,
  'deletionState': _$UploadDeletionStateEnumEnumMap[instance.deletionState],
  'retentionHeld': instance.retentionHeld,
  'retentionState': _$UploadRetentionStateEnumEnumMap[instance.retentionState]!,
  'retentionExpiresAt': instance.retentionExpiresAt,
  'createdAt': instance.createdAt,
};

const _$UploadKindEnumEnumMap = {
  UploadKindEnum.image: 'image',
  UploadKindEnum.file: 'file',
  UploadKindEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$UploadStatusEnumEnumMap = {
  UploadStatusEnum.pending: 'pending',
  UploadStatusEnum.clean: 'clean',
  UploadStatusEnum.quarantined: 'quarantined',
  UploadStatusEnum.blocked: 'blocked',
  UploadStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$MediaDeliveryStateEnumMap = {
  MediaDeliveryState.unpublished: 'unpublished',
  MediaDeliveryState.processing: 'processing',
  MediaDeliveryState.published: 'published',
  MediaDeliveryState.failed: 'failed',
  MediaDeliveryState.blocked: 'blocked',
  MediaDeliveryState.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$UploadDeliveryErrorCodeEnumEnumMap = {
  UploadDeliveryErrorCodeEnum.legacyAnimatedFormatRequiresReupload:
      'legacy_animated_format_requires_reupload',
  UploadDeliveryErrorCodeEnum.leaseExpiredAfterMaxAttempts:
      'lease_expired_after_max_attempts',
  UploadDeliveryErrorCodeEnum.assetLeftCleanState: 'asset_left_clean_state',
  UploadDeliveryErrorCodeEnum.invalidSourceLength: 'invalid_source_length',
  UploadDeliveryErrorCodeEnum.ingestReadFailed: 'ingest_read_failed',
  UploadDeliveryErrorCodeEnum.sourceDigestMismatch: 'source_digest_mismatch',
  UploadDeliveryErrorCodeEnum.imageWorkerJoinFailed: 'image_worker_join_failed',
  UploadDeliveryErrorCodeEnum.imageDecodeRejected: 'image_decode_rejected',
  UploadDeliveryErrorCodeEnum.variantRegistrationFailed:
      'variant_registration_failed',
  UploadDeliveryErrorCodeEnum.deliveryWriteFailed: 'delivery_write_failed',
  UploadDeliveryErrorCodeEnum.deliveryVerificationFailed:
      'delivery_verification_failed',
  UploadDeliveryErrorCodeEnum.publicationCommitFailed:
      'publication_commit_failed',
  UploadDeliveryErrorCodeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$MediaUsageEnumMap = {
  MediaUsage.profileAvatar: 'profile_avatar',
  MediaUsage.profileBanner: 'profile_banner',
  MediaUsage.forumThread: 'forum_thread',
  MediaUsage.forumComment: 'forum_comment',
  MediaUsage.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$UploadApprovalRequirementEnumEnumMap = {
  UploadApprovalRequirementEnum.none: 'none',
  UploadApprovalRequirementEnum.imagePreview: 'image_preview',
  UploadApprovalRequirementEnum.scanner: 'scanner',
  UploadApprovalRequirementEnum.satisfied: 'satisfied',
  UploadApprovalRequirementEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$UploadDeletionStateEnumEnumMap = {
  UploadDeletionStateEnum.queued: 'queued',
  UploadDeletionStateEnum.leased: 'leased',
  UploadDeletionStateEnum.succeeded: 'succeeded',
  UploadDeletionStateEnum.deadLetter: 'dead_letter',
  UploadDeletionStateEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$UploadRetentionStateEnumEnumMap = {
  UploadRetentionStateEnum.none: 'none',
  UploadRetentionStateEnum.active: 'active',
  UploadRetentionStateEnum.expired: 'expired',
  UploadRetentionStateEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
