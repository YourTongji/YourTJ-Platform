// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'achievement_event_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AchievementEventPage _$AchievementEventPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AchievementEventPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = AchievementEventPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => AchievementEvent.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$AchievementEventPageToJson(
  AchievementEventPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
