// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_forum_flag_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminForumFlagPage _$AdminForumFlagPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminForumFlagPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = AdminForumFlagPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => AdminForumFlag.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$AdminForumFlagPageToJson(AdminForumFlagPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
