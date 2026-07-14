// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_appeal_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminAppealPage _$AdminAppealPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminAppealPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = AdminAppealPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => AdminAppeal.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$AdminAppealPageToJson(AdminAppealPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
