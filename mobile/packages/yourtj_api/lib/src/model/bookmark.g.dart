// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'bookmark.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Bookmark _$BookmarkFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Bookmark', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'targetType',
          'targetId',
          'note',
          'createdAt',
          'content',
        ],
      );
      final val = Bookmark(
        targetType: $checkedConvert(
          'targetType',
          (v) => $enumDecode(
            _$BookmarkTargetTypeEnumEnumMap,
            v,
            unknownValue: BookmarkTargetTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        targetId: $checkedConvert('targetId', (v) => v as String),
        note: $checkedConvert('note', (v) => v as String?),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
        content: $checkedConvert(
          'content',
          (v) => ProfileContent.fromJson(v as Map<String, dynamic>),
        ),
      );
      return val;
    });

Map<String, dynamic> _$BookmarkToJson(Bookmark instance) => <String, dynamic>{
  'targetType': _$BookmarkTargetTypeEnumEnumMap[instance.targetType]!,
  'targetId': instance.targetId,
  'note': instance.note,
  'createdAt': instance.createdAt,
  'content': instance.content.toJson(),
};

const _$BookmarkTargetTypeEnumEnumMap = {
  BookmarkTargetTypeEnum.thread: 'thread',
  BookmarkTargetTypeEnum.comment: 'comment',
  BookmarkTargetTypeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
