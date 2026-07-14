// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_user_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminUserPage _$AdminUserPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminUserPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = AdminUserPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => AdminUser.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$AdminUserPageToJson(AdminUserPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
