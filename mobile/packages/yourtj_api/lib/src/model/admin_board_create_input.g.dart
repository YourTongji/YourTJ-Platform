// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_board_create_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminBoardCreateInput _$AdminBoardCreateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminBoardCreateInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['slug', 'name', 'reason']);
  final val = AdminBoardCreateInput(
    slug: $checkedConvert('slug', (v) => v as String),
    name: $checkedConvert('name', (v) => v as String),
    description: $checkedConvert('description', (v) => v as String?),
    position: $checkedConvert('position', (v) => (v as num?)?.toInt() ?? 0),
    isLocked: $checkedConvert('isLocked', (v) => v as bool? ?? false),
    minTrustToPost: $checkedConvert(
      'minTrustToPost',
      (v) => (v as num?)?.toInt() ?? 1,
    ),
    isQa: $checkedConvert('isQa', (v) => v as bool? ?? false),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AdminBoardCreateInputToJson(
  AdminBoardCreateInput instance,
) => <String, dynamic>{
  'slug': instance.slug,
  'name': instance.name,
  'description': ?instance.description,
  'position': ?instance.position,
  'isLocked': ?instance.isLocked,
  'minTrustToPost': ?instance.minTrustToPost,
  'isQa': ?instance.isQa,
  'reason': instance.reason,
};
