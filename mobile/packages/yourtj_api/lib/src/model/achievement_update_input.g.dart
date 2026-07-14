// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AchievementUpdateInput _$AchievementUpdateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AchievementUpdateInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'expectedVersion',
      'name',
      'icon',
      'status',
      'mintAmount',
      'reason',
    ],
  );
  final val = AchievementUpdateInput(
    expectedVersion: $checkedConvert(
      'expectedVersion',
      (v) => (v as num).toInt(),
    ),
    name: $checkedConvert('name', (v) => v as String),
    description: $checkedConvert('description', (v) => v as String?),
    icon: $checkedConvert(
      'icon',
      (v) => $enumDecode(
        _$AchievementIconEnumMap,
        v,
        unknownValue: AchievementIcon.unknownDefaultOpenApi,
      ),
    ),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$AchievementStatusEnumMap,
        v,
        unknownValue: AchievementStatus.unknownDefaultOpenApi,
      ),
    ),
    mintAmount: $checkedConvert('mintAmount', (v) => (v as num).toInt()),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AchievementUpdateInputToJson(
  AchievementUpdateInput instance,
) => <String, dynamic>{
  'expectedVersion': instance.expectedVersion,
  'name': instance.name,
  'description': ?instance.description,
  'icon': _$AchievementIconEnumMap[instance.icon]!,
  'status': _$AchievementStatusEnumMap[instance.status]!,
  'mintAmount': instance.mintAmount,
  'reason': instance.reason,
};

const _$AchievementIconEnumMap = {
  AchievementIcon.award: 'award',
  AchievementIcon.bookOpenCheck: 'book-open-check',
  AchievementIcon.messageCircleHeart: 'message-circle-heart',
  AchievementIcon.star: 'star',
  AchievementIcon.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AchievementStatusEnumMap = {
  AchievementStatus.active: 'active',
  AchievementStatus.retired: 'retired',
  AchievementStatus.unknownDefaultOpenApi: 'unknown_default_open_api',
};
