// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'bookmark_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

BookmarkInput _$BookmarkInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('BookmarkInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['postType']);
      final val = BookmarkInput(
        postType: $checkedConvert(
          'postType',
          (v) => $enumDecode(
            _$BookmarkInputPostTypeEnumEnumMap,
            v,
            unknownValue: BookmarkInputPostTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        note: $checkedConvert('note', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$BookmarkInputToJson(BookmarkInput instance) =>
    <String, dynamic>{
      'postType': _$BookmarkInputPostTypeEnumEnumMap[instance.postType]!,
      'note': ?instance.note,
    };

const _$BookmarkInputPostTypeEnumEnumMap = {
  BookmarkInputPostTypeEnum.thread: 'thread',
  BookmarkInputPostTypeEnum.comment: 'comment',
  BookmarkInputPostTypeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
