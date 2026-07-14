// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement_revoke_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AchievementRevokeInput _$AchievementRevokeInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AchievementRevokeInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = AchievementRevokeInput(
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AchievementRevokeInputToJson(
  AchievementRevokeInput instance,
) => <String, dynamic>{'reason': instance.reason};
