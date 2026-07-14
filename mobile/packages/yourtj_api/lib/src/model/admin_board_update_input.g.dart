// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_board_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminBoardUpdateInput _$AdminBoardUpdateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminBoardUpdateInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = AdminBoardUpdateInput(
    slug: $checkedConvert('slug', (v) => v as String?),
    name: $checkedConvert('name', (v) => v as String?),
    description: $checkedConvert('description', (v) => v as String?),
    position: $checkedConvert('position', (v) => (v as num?)?.toInt()),
    isLocked: $checkedConvert('isLocked', (v) => v as bool?),
    minTrustToPost: $checkedConvert(
      'minTrustToPost',
      (v) => (v as num?)?.toInt(),
    ),
    isQa: $checkedConvert('isQa', (v) => v as bool?),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AdminBoardUpdateInputToJson(
  AdminBoardUpdateInput instance,
) => <String, dynamic>{
  'slug': ?instance.slug,
  'name': ?instance.name,
  'description': ?instance.description,
  'position': ?instance.position,
  'isLocked': ?instance.isLocked,
  'minTrustToPost': ?instance.minTrustToPost,
  'isQa': ?instance.isQa,
  'reason': instance.reason,
};
