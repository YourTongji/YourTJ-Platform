// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'announcement_revision_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AnnouncementRevisionPage _$AnnouncementRevisionPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AnnouncementRevisionPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = AnnouncementRevisionPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => AnnouncementRevision.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$AnnouncementRevisionPageToJson(
  AnnouncementRevisionPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
