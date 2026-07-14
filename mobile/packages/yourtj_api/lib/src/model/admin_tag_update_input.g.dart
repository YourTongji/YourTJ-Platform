// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_tag_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminTagUpdateInput _$AdminTagUpdateInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminTagUpdateInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['reason']);
      final val = AdminTagUpdateInput(
        slug: $checkedConvert('slug', (v) => v as String?),
        name: $checkedConvert('name', (v) => v as String?),
        description: $checkedConvert('description', (v) => v as String?),
        reason: $checkedConvert('reason', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$AdminTagUpdateInputToJson(
  AdminTagUpdateInput instance,
) => <String, dynamic>{
  'slug': ?instance.slug,
  'name': ?instance.name,
  'description': ?instance.description,
  'reason': instance.reason,
};
