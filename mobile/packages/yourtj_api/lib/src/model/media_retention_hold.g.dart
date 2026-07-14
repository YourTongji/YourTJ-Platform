// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_retention_hold.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaRetentionHold _$MediaRetentionHoldFromJson(Map<String, dynamic> json) =>
    $checkedCreate('MediaRetentionHold', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'uploadId',
          'accountId',
          'uploadStatus',
          'holdKind',
          'reason',
          'placedBy',
          'expiresAt',
          'createdAt',
          'isExpired',
        ],
      );
      final val = MediaRetentionHold(
        id: $checkedConvert('id', (v) => v as String),
        uploadId: $checkedConvert('uploadId', (v) => v as String),
        accountId: $checkedConvert('accountId', (v) => v as String),
        uploadStatus: $checkedConvert(
          'uploadStatus',
          (v) => $enumDecode(
            _$MediaRetentionHoldUploadStatusEnumEnumMap,
            v,
            unknownValue:
                MediaRetentionHoldUploadStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        holdKind: $checkedConvert(
          'holdKind',
          (v) => $enumDecode(
            _$MediaRetentionHoldHoldKindEnumEnumMap,
            v,
            unknownValue: MediaRetentionHoldHoldKindEnum.unknownDefaultOpenApi,
          ),
        ),
        reason: $checkedConvert('reason', (v) => v as String),
        placedBy: $checkedConvert('placedBy', (v) => v as String),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
        isExpired: $checkedConvert('isExpired', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$MediaRetentionHoldToJson(MediaRetentionHold instance) =>
    <String, dynamic>{
      'id': instance.id,
      'uploadId': instance.uploadId,
      'accountId': instance.accountId,
      'uploadStatus':
          _$MediaRetentionHoldUploadStatusEnumEnumMap[instance.uploadStatus]!,
      'holdKind': _$MediaRetentionHoldHoldKindEnumEnumMap[instance.holdKind]!,
      'reason': instance.reason,
      'placedBy': instance.placedBy,
      'expiresAt': instance.expiresAt,
      'createdAt': instance.createdAt,
      'isExpired': instance.isExpired,
    };

const _$MediaRetentionHoldUploadStatusEnumEnumMap = {
  MediaRetentionHoldUploadStatusEnum.pending: 'pending',
  MediaRetentionHoldUploadStatusEnum.clean: 'clean',
  MediaRetentionHoldUploadStatusEnum.quarantined: 'quarantined',
  MediaRetentionHoldUploadStatusEnum.blocked: 'blocked',
  MediaRetentionHoldUploadStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$MediaRetentionHoldHoldKindEnumEnumMap = {
  MediaRetentionHoldHoldKindEnum.moderation: 'moderation',
  MediaRetentionHoldHoldKindEnum.security: 'security',
  MediaRetentionHoldHoldKindEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
