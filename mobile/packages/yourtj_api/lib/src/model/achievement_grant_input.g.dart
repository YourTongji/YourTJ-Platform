// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement_grant_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AchievementGrantInput _$AchievementGrantInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AchievementGrantInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['achievementId', 'reason']);
  final val = AchievementGrantInput(
    achievementId: $checkedConvert('achievementId', (v) => v as String),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AchievementGrantInputToJson(
  AchievementGrantInput instance,
) => <String, dynamic>{
  'achievementId': instance.achievementId,
  'reason': instance.reason,
};
