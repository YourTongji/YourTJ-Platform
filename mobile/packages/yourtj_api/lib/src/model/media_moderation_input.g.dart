// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_moderation_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaModerationInput _$MediaModerationInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('MediaModerationInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = MediaModerationInput(
    reason: $checkedConvert('reason', (v) => v as String),
    selfReviewConfirmed: $checkedConvert(
      'selfReviewConfirmed',
      (v) => v as bool? ?? false,
    ),
  );
  return val;
});

Map<String, dynamic> _$MediaModerationInputToJson(
  MediaModerationInput instance,
) => <String, dynamic>{
  'reason': instance.reason,
  'selfReviewConfirmed': ?instance.selfReviewConfirmed,
};
