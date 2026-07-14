// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'comment_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CommentPage _$CommentPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('CommentPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = CommentPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Comment.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$CommentPageToJson(CommentPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
