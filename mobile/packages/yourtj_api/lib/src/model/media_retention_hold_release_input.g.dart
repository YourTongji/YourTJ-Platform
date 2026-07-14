// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_retention_hold_release_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaRetentionHoldReleaseInput _$MediaRetentionHoldReleaseInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('MediaRetentionHoldReleaseInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['expectedHoldId', 'reason']);
  final val = MediaRetentionHoldReleaseInput(
    expectedHoldId: $checkedConvert('expectedHoldId', (v) => v as String),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$MediaRetentionHoldReleaseInputToJson(
  MediaRetentionHoldReleaseInput instance,
) => <String, dynamic>{
  'expectedHoldId': instance.expectedHoldId,
  'reason': instance.reason,
};
