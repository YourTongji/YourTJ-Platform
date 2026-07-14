// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'bookmark_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

BookmarkPage _$BookmarkPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('BookmarkPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = BookmarkPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Bookmark.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$BookmarkPageToJson(BookmarkPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
