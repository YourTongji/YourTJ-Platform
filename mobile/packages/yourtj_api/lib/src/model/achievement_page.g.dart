// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AchievementPage _$AchievementPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AchievementPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = AchievementPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Achievement.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$AchievementPageToJson(AchievementPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
