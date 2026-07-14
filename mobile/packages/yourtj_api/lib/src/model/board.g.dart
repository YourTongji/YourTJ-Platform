// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'board.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Board _$BoardFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Board', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'slug',
          'name',
          'parentId',
          'description',
          'position',
          'isLocked',
          'minTrustToPost',
          'isQa',
          'threadCount',
          'canPost',
          'postingRestriction',
        ],
      );
      final val = Board(
        id: $checkedConvert('id', (v) => v as String),
        slug: $checkedConvert('slug', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
        parentId: $checkedConvert('parentId', (v) => v as String?),
        description: $checkedConvert('description', (v) => v as String?),
        position: $checkedConvert('position', (v) => (v as num).toInt()),
        isLocked: $checkedConvert('isLocked', (v) => v as bool),
        minTrustToPost: $checkedConvert(
          'minTrustToPost',
          (v) => (v as num).toInt(),
        ),
        isQa: $checkedConvert('isQa', (v) => v as bool),
        threadCount: $checkedConvert('threadCount', (v) => (v as num).toInt()),
        canPost: $checkedConvert('canPost', (v) => v as bool),
        postingRestriction: $checkedConvert(
          'postingRestriction',
          (v) => $enumDecodeNullable(
            _$BoardPostingRestrictionEnumEnumMap,
            v,
            unknownValue: BoardPostingRestrictionEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$BoardToJson(Board instance) => <String, dynamic>{
  'id': instance.id,
  'slug': instance.slug,
  'name': instance.name,
  'parentId': instance.parentId,
  'description': instance.description,
  'position': instance.position,
  'isLocked': instance.isLocked,
  'minTrustToPost': instance.minTrustToPost,
  'isQa': instance.isQa,
  'threadCount': instance.threadCount,
  'canPost': instance.canPost,
  'postingRestriction':
      _$BoardPostingRestrictionEnumEnumMap[instance.postingRestriction],
};

const _$BoardPostingRestrictionEnumEnumMap = {
  BoardPostingRestrictionEnum.loginRequired: 'login_required',
  BoardPostingRestrictionEnum.boardLocked: 'board_locked',
  BoardPostingRestrictionEnum.trustLevel: 'trust_level',
  BoardPostingRestrictionEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
