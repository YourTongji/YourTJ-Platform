// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trust_level_event_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TrustLevelEventPage _$TrustLevelEventPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('TrustLevelEventPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = TrustLevelEventPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => TrustLevelEvent.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$TrustLevelEventPageToJson(
  TrustLevelEventPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
