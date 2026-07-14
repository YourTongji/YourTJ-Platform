//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/media_delivery_state.dart';
import 'package:yourtj_api/src/model/media_usage.dart';
import 'package:json_annotation/json_annotation.dart';

part 'upload.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Upload {
  /// Returns a new [Upload] instance.
  Upload({
    required this.id,

    required this.accountId,

    required this.kind,

    required this.bytes,

    required this.mime,

    required this.status,

    required this.deliveryState,

    required this.deliveryErrorCode,

    required this.usage,

    required this.imageWidth,

    required this.imageHeight,

    required this.isSelfReview,

    required this.approvalRequirement,

    required this.deletionState,

    required this.retentionHeld,

    required this.retentionState,

    required this.retentionExpiresAt,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(
    name: r'kind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: UploadKindEnum.unknownDefaultOpenApi,
  )
  final UploadKindEnum kind;

  @JsonKey(name: r'bytes', required: true, includeIfNull: false)
  final int bytes;

  @JsonKey(name: r'mime', required: true, includeIfNull: false)
  final String mime;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: UploadStatusEnum.unknownDefaultOpenApi,
  )
  final UploadStatusEnum status;

  @JsonKey(
    name: r'deliveryState',
    required: true,
    includeIfNull: false,
    unknownEnumValue: MediaDeliveryState.unknownDefaultOpenApi,
  )
  final MediaDeliveryState deliveryState;

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonKey(
    name: r'deliveryErrorCode',
    required: true,
    includeIfNull: true,
    unknownEnumValue: UploadDeliveryErrorCodeEnum.unknownDefaultOpenApi,
  )
  final UploadDeliveryErrorCodeEnum? deliveryErrorCode;

  @JsonKey(
    name: r'usage',
    required: true,
    includeIfNull: true,
    unknownEnumValue: MediaUsage.unknownDefaultOpenApi,
  )
  final MediaUsage? usage;

  // minimum: 1
  // maximum: 20000
  @JsonKey(name: r'imageWidth', required: true, includeIfNull: true)
  final int? imageWidth;

  // minimum: 1
  // maximum: 20000
  @JsonKey(name: r'imageHeight', required: true, includeIfNull: true)
  final int? imageHeight;

  /// True only for the current ADMIN's own upload shown under the explicit self-review exception.
  @JsonKey(name: r'isSelfReview', required: true, includeIfNull: false)
  final bool isSelfReview;

  /// Actor-specific evidence gate. Files remain scanner-gated; image approval requires this moderator's trusted preview.
  @JsonKey(
    name: r'approvalRequirement',
    required: true,
    includeIfNull: false,
    unknownEnumValue: UploadApprovalRequirementEnum.unknownDefaultOpenApi,
  )
  final UploadApprovalRequirementEnum approvalRequirement;

  /// Durable provider deletion state, present after an upload enters quarantine.
  @JsonKey(
    name: r'deletionState',
    required: true,
    includeIfNull: true,
    unknownEnumValue: UploadDeletionStateEnum.unknownDefaultOpenApi,
  )
  final UploadDeletionStateEnum? deletionState;

  /// Whether a purpose-bound, unexpired operations hold currently pauses deletion. The reason and kind are never disclosed here.
  @JsonKey(name: r'retentionHeld', required: true, includeIfNull: false)
  final bool retentionHeld;

  /// Presence state of the unreleased operations record; expired records must be reviewed in the operations inventory before replacement or release.
  @JsonKey(
    name: r'retentionState',
    required: true,
    includeIfNull: false,
    unknownEnumValue: UploadRetentionStateEnum.unknownDefaultOpenApi,
  )
  final UploadRetentionStateEnum retentionState;

  /// Unix seconds for the unreleased operations record; null only when retentionState is none.
  @JsonKey(name: r'retentionExpiresAt', required: true, includeIfNull: true)
  final int? retentionExpiresAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Upload &&
          other.id == id &&
          other.accountId == accountId &&
          other.kind == kind &&
          other.bytes == bytes &&
          other.mime == mime &&
          other.status == status &&
          other.deliveryState == deliveryState &&
          other.deliveryErrorCode == deliveryErrorCode &&
          other.usage == usage &&
          other.imageWidth == imageWidth &&
          other.imageHeight == imageHeight &&
          other.isSelfReview == isSelfReview &&
          other.approvalRequirement == approvalRequirement &&
          other.deletionState == deletionState &&
          other.retentionHeld == retentionHeld &&
          other.retentionState == retentionState &&
          other.retentionExpiresAt == retentionExpiresAt &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      accountId.hashCode +
      kind.hashCode +
      bytes.hashCode +
      mime.hashCode +
      status.hashCode +
      deliveryState.hashCode +
      (deliveryErrorCode == null ? 0 : deliveryErrorCode.hashCode) +
      (usage == null ? 0 : usage.hashCode) +
      (imageWidth == null ? 0 : imageWidth.hashCode) +
      (imageHeight == null ? 0 : imageHeight.hashCode) +
      isSelfReview.hashCode +
      approvalRequirement.hashCode +
      (deletionState == null ? 0 : deletionState.hashCode) +
      retentionHeld.hashCode +
      retentionState.hashCode +
      (retentionExpiresAt == null ? 0 : retentionExpiresAt.hashCode) +
      createdAt.hashCode;

  factory Upload.fromJson(Map<String, dynamic> json) => _$UploadFromJson(json);

  Map<String, dynamic> toJson() => _$UploadToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum UploadKindEnum {
  @JsonValue(r'image')
  image(r'image'),
  @JsonValue(r'file')
  file(r'file'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UploadKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum UploadStatusEnum {
  @JsonValue(r'pending')
  pending(r'pending'),
  @JsonValue(r'clean')
  clean(r'clean'),
  @JsonValue(r'quarantined')
  quarantined(r'quarantined'),
  @JsonValue(r'blocked')
  blocked(r'blocked'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UploadStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

/// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
enum UploadDeliveryErrorCodeEnum {
  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'legacy_animated_format_requires_reupload')
  legacyAnimatedFormatRequiresReupload(
    r'legacy_animated_format_requires_reupload',
  ),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'lease_expired_after_max_attempts')
  leaseExpiredAfterMaxAttempts(r'lease_expired_after_max_attempts'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'asset_left_clean_state')
  assetLeftCleanState(r'asset_left_clean_state'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'invalid_source_length')
  invalidSourceLength(r'invalid_source_length'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'ingest_read_failed')
  ingestReadFailed(r'ingest_read_failed'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'source_digest_mismatch')
  sourceDigestMismatch(r'source_digest_mismatch'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'image_worker_join_failed')
  imageWorkerJoinFailed(r'image_worker_join_failed'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'image_decode_rejected')
  imageDecodeRejected(r'image_decode_rejected'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'variant_registration_failed')
  variantRegistrationFailed(r'variant_registration_failed'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'delivery_write_failed')
  deliveryWriteFailed(r'delivery_write_failed'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'delivery_verification_failed')
  deliveryVerificationFailed(r'delivery_verification_failed'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'publication_commit_failed')
  publicationCommitFailed(r'publication_commit_failed'),

  /// Bounded operations-only failure category; it never contains a provider URL, key, response body, or user data.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UploadDeliveryErrorCodeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

/// Actor-specific evidence gate. Files remain scanner-gated; image approval requires this moderator's trusted preview.
enum UploadApprovalRequirementEnum {
  /// Actor-specific evidence gate. Files remain scanner-gated; image approval requires this moderator's trusted preview.
  @JsonValue(r'none')
  none(r'none'),

  /// Actor-specific evidence gate. Files remain scanner-gated; image approval requires this moderator's trusted preview.
  @JsonValue(r'image_preview')
  imagePreview(r'image_preview'),

  /// Actor-specific evidence gate. Files remain scanner-gated; image approval requires this moderator's trusted preview.
  @JsonValue(r'scanner')
  scanner(r'scanner'),

  /// Actor-specific evidence gate. Files remain scanner-gated; image approval requires this moderator's trusted preview.
  @JsonValue(r'satisfied')
  satisfied(r'satisfied'),

  /// Actor-specific evidence gate. Files remain scanner-gated; image approval requires this moderator's trusted preview.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UploadApprovalRequirementEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

/// Durable provider deletion state, present after an upload enters quarantine.
enum UploadDeletionStateEnum {
  /// Durable provider deletion state, present after an upload enters quarantine.
  @JsonValue(r'queued')
  queued(r'queued'),

  /// Durable provider deletion state, present after an upload enters quarantine.
  @JsonValue(r'leased')
  leased(r'leased'),

  /// Durable provider deletion state, present after an upload enters quarantine.
  @JsonValue(r'succeeded')
  succeeded(r'succeeded'),

  /// Durable provider deletion state, present after an upload enters quarantine.
  @JsonValue(r'dead_letter')
  deadLetter(r'dead_letter'),

  /// Durable provider deletion state, present after an upload enters quarantine.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UploadDeletionStateEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

/// Presence state of the unreleased operations record; expired records must be reviewed in the operations inventory before replacement or release.
enum UploadRetentionStateEnum {
  /// Presence state of the unreleased operations record; expired records must be reviewed in the operations inventory before replacement or release.
  @JsonValue(r'none')
  none(r'none'),

  /// Presence state of the unreleased operations record; expired records must be reviewed in the operations inventory before replacement or release.
  @JsonValue(r'active')
  active(r'active'),

  /// Presence state of the unreleased operations record; expired records must be reviewed in the operations inventory before replacement or release.
  @JsonValue(r'expired')
  expired(r'expired'),

  /// Presence state of the unreleased operations record; expired records must be reviewed in the operations inventory before replacement or release.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UploadRetentionStateEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
