// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_retention_hold_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaRetentionHoldInput _$MediaRetentionHoldInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('MediaRetentionHoldInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const ['holdKind', 'expiresAt', 'reason', 'expectedHoldId'],
  );
  final val = MediaRetentionHoldInput(
    holdKind: $checkedConvert(
      'holdKind',
      (v) => $enumDecode(
        _$MediaRetentionHoldInputHoldKindEnumEnumMap,
        v,
        unknownValue: MediaRetentionHoldInputHoldKindEnum.unknownDefaultOpenApi,
      ),
    ),
    expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
    reason: $checkedConvert('reason', (v) => v as String),
    expectedHoldId: $checkedConvert('expectedHoldId', (v) => v as String?),
  );
  return val;
});

Map<String, dynamic> _$MediaRetentionHoldInputToJson(
  MediaRetentionHoldInput instance,
) => <String, dynamic>{
  'holdKind': _$MediaRetentionHoldInputHoldKindEnumEnumMap[instance.holdKind]!,
  'expiresAt': instance.expiresAt,
  'reason': instance.reason,
  'expectedHoldId': instance.expectedHoldId,
};

const _$MediaRetentionHoldInputHoldKindEnumEnumMap = {
  MediaRetentionHoldInputHoldKindEnum.moderation: 'moderation',
  MediaRetentionHoldInputHoldKindEnum.security: 'security',
  MediaRetentionHoldInputHoldKindEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
