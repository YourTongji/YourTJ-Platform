// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement_create_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AchievementCreateInput _$AchievementCreateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AchievementCreateInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const ['slug', 'name', 'icon', 'mintAmount', 'reason'],
  );
  final val = AchievementCreateInput(
    slug: $checkedConvert('slug', (v) => v as String),
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
    mintAmount: $checkedConvert('mintAmount', (v) => (v as num).toInt()),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AchievementCreateInputToJson(
  AchievementCreateInput instance,
) => <String, dynamic>{
  'slug': instance.slug,
  'name': instance.name,
  'description': ?instance.description,
  'icon': _$AchievementIconEnumMap[instance.icon]!,
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
