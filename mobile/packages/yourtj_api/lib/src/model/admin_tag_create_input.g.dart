// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_tag_create_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminTagCreateInput _$AdminTagCreateInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminTagCreateInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['slug', 'name', 'reason']);
      final val = AdminTagCreateInput(
        slug: $checkedConvert('slug', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
        description: $checkedConvert('description', (v) => v as String?),
        reason: $checkedConvert('reason', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$AdminTagCreateInputToJson(
  AdminTagCreateInput instance,
) => <String, dynamic>{
  'slug': instance.slug,
  'name': instance.name,
  'description': ?instance.description,
  'reason': instance.reason,
};
