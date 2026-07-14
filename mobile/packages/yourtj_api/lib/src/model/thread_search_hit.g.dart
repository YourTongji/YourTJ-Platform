// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_search_hit.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadSearchHit _$ThreadSearchHitFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ThreadSearchHit', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'title',
          'bodyExcerpt',
          'board',
          'tags',
          'authorHandle',
          'replyCount',
          'voteCount',
          'createdAt',
          'status',
        ],
      );
      final val = ThreadSearchHit(
        id: $checkedConvert('id', (v) => v as String),
        title: $checkedConvert('title', (v) => v as String),
        bodyExcerpt: $checkedConvert('bodyExcerpt', (v) => v as String),
        board: $checkedConvert('board', (v) => v as String),
        tags: $checkedConvert(
          'tags',
          (v) => (v as List<dynamic>).map((e) => e as String).toList(),
        ),
        authorHandle: $checkedConvert('authorHandle', (v) => v as String),
        replyCount: $checkedConvert('replyCount', (v) => (v as num).toInt()),
        voteCount: $checkedConvert('voteCount', (v) => (v as num).toInt()),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$ThreadSearchHitStatusEnumEnumMap,
            v,
            unknownValue: ThreadSearchHitStatusEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$ThreadSearchHitToJson(ThreadSearchHit instance) =>
    <String, dynamic>{
      'id': instance.id,
      'title': instance.title,
      'bodyExcerpt': instance.bodyExcerpt,
      'board': instance.board,
      'tags': instance.tags,
      'authorHandle': instance.authorHandle,
      'replyCount': instance.replyCount,
      'voteCount': instance.voteCount,
      'createdAt': instance.createdAt,
      'status': _$ThreadSearchHitStatusEnumEnumMap[instance.status]!,
    };

const _$ThreadSearchHitStatusEnumEnumMap = {
  ThreadSearchHitStatusEnum.visible: 'visible',
  ThreadSearchHitStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
