// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_feed.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadFeed _$ThreadFeedFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ThreadFeed', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'boardId',
      'authorHandle',
      'authorAvatar',
      'title',
      'bodyExcerpt',
      'contentVersion',
      'replyCount',
      'voteCount',
      'hotScore',
      'status',
      'createdAt',
      'lastActivityAt',
      'tags',
      'attachments',
      'viewerVote',
      'isBookmarked',
      'canEdit',
      'canDelete',
      'canModerate',
    ],
  );
  final val = ThreadFeed(
    id: $checkedConvert('id', (v) => v as String),
    boardId: $checkedConvert('boardId', (v) => v as String),
    authorHandle: $checkedConvert('authorHandle', (v) => v as String),
    authorDisplayName: $checkedConvert(
      'authorDisplayName',
      (v) => v as String?,
    ),
    authorAvatar: $checkedConvert(
      'authorAvatar',
      (v) =>
          v == null ? null : MediaDelivery.fromJson(v as Map<String, dynamic>),
    ),
    title: $checkedConvert('title', (v) => v as String),
    bodyExcerpt: $checkedConvert('bodyExcerpt', (v) => v as String?),
    contentVersion: $checkedConvert(
      'contentVersion',
      (v) => (v as num).toInt(),
    ),
    replyCount: $checkedConvert('replyCount', (v) => (v as num).toInt()),
    voteCount: $checkedConvert('voteCount', (v) => (v as num).toInt()),
    hotScore: $checkedConvert('hotScore', (v) => v as num?),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$ThreadFeedStatusEnumEnumMap,
        v,
        unknownValue: ThreadFeedStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
    lastActivityAt: $checkedConvert(
      'lastActivityAt',
      (v) => (v as num).toInt(),
    ),
    tags: $checkedConvert(
      'tags',
      (v) => (v as List<dynamic>).map((e) => e as String).toList(),
    ),
    attachments: $checkedConvert(
      'attachments',
      (v) => (v as List<dynamic>)
          .map((e) => ForumAttachment.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    viewerVote: $checkedConvert(
      'viewerVote',
      (v) => $enumDecodeNullable(
        _$ThreadFeedViewerVoteEnumEnumMap,
        v,
        unknownValue: ThreadFeedViewerVoteEnum.unknownDefaultOpenApi,
      ),
    ),
    isBookmarked: $checkedConvert('isBookmarked', (v) => v as bool),
    canEdit: $checkedConvert('canEdit', (v) => v as bool),
    canDelete: $checkedConvert('canDelete', (v) => v as bool),
    canModerate: $checkedConvert('canModerate', (v) => v as bool),
    unreadCount: $checkedConvert('unreadCount', (v) => (v as num?)?.toInt()),
  );
  return val;
});

Map<String, dynamic> _$ThreadFeedToJson(ThreadFeed instance) =>
    <String, dynamic>{
      'id': instance.id,
      'boardId': instance.boardId,
      'authorHandle': instance.authorHandle,
      'authorDisplayName': ?instance.authorDisplayName,
      'authorAvatar': instance.authorAvatar?.toJson(),
      'title': instance.title,
      'bodyExcerpt': instance.bodyExcerpt,
      'contentVersion': instance.contentVersion,
      'replyCount': instance.replyCount,
      'voteCount': instance.voteCount,
      'hotScore': instance.hotScore,
      'status': _$ThreadFeedStatusEnumEnumMap[instance.status]!,
      'createdAt': instance.createdAt,
      'lastActivityAt': instance.lastActivityAt,
      'tags': instance.tags,
      'attachments': instance.attachments.map((e) => e.toJson()).toList(),
      'viewerVote': _$ThreadFeedViewerVoteEnumEnumMap[instance.viewerVote],
      'isBookmarked': instance.isBookmarked,
      'canEdit': instance.canEdit,
      'canDelete': instance.canDelete,
      'canModerate': instance.canModerate,
      'unreadCount': ?instance.unreadCount,
    };

const _$ThreadFeedStatusEnumEnumMap = {
  ThreadFeedStatusEnum.visible: 'visible',
  ThreadFeedStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$ThreadFeedViewerVoteEnumEnumMap = {
  ThreadFeedViewerVoteEnum.up: 'up',
  ThreadFeedViewerVoteEnum.down: 'down',
  ThreadFeedViewerVoteEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
