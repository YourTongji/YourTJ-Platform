// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_comment_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserCommentPage _$UserCommentPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UserCommentPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = UserCommentPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => UserComment.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$UserCommentPageToJson(UserCommentPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
