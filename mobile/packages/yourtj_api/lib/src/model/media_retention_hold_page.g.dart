// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_retention_hold_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaRetentionHoldPage _$MediaRetentionHoldPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('MediaRetentionHoldPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = MediaRetentionHoldPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => MediaRetentionHold.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$MediaRetentionHoldPageToJson(
  MediaRetentionHoldPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
