// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement_grant_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AchievementGrantPage _$AchievementGrantPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AchievementGrantPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = AchievementGrantPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => AchievementGrant.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$AchievementGrantPageToJson(
  AchievementGrantPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
