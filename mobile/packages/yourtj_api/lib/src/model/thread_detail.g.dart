// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_detail.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadDetail _$ThreadDetailFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ThreadDetail', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'boardId',
      'authorHandle',
      'authorAvatar',
      'authorId',
      'title',
      'body',
      'contentFormat',
      'contentVersion',
      'replyCount',
      'voteCount',
      'hotScore',
      'tags',
      'attachments',
      'status',
      'pinnedAt',
      'pinnedGlobally',
      'featuredAt',
      'closedAt',
      'archivedAt',
      'deletedAt',
      'editedAt',
      'hiddenAt',
      'createdAt',
      'lastActivityAt',
      'solvedAnswerId',
      'viewerVote',
      'isBookmarked',
      'myLastReadCommentId',
      'mySubscriptionLevel',
      'poll',
      'canEdit',
      'canDelete',
      'canModerate',
    ],
  );
  final val = ThreadDetail(
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
    authorId: $checkedConvert('authorId', (v) => v as String),
    title: $checkedConvert('title', (v) => v as String),
    body: $checkedConvert('body', (v) => v as String?),
    contentFormat: $checkedConvert(
      'contentFormat',
      (v) => $enumDecode(
        _$ContentFormatEnumMap,
        v,
        unknownValue: ContentFormat.unknownDefaultOpenApi,
      ),
    ),
    contentVersion: $checkedConvert(
      'contentVersion',
      (v) => (v as num).toInt(),
    ),
    replyCount: $checkedConvert('replyCount', (v) => (v as num).toInt()),
    voteCount: $checkedConvert('voteCount', (v) => (v as num).toInt()),
    hotScore: $checkedConvert('hotScore', (v) => v as num?),
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
    status: $checkedConvert('status', (v) => v as String),
    pinnedAt: $checkedConvert('pinnedAt', (v) => (v as num?)?.toInt()),
    pinnedGlobally: $checkedConvert('pinnedGlobally', (v) => v as bool),
    featuredAt: $checkedConvert('featuredAt', (v) => (v as num?)?.toInt()),
    closedAt: $checkedConvert('closedAt', (v) => (v as num?)?.toInt()),
    archivedAt: $checkedConvert('archivedAt', (v) => (v as num?)?.toInt()),
    deletedAt: $checkedConvert('deletedAt', (v) => (v as num?)?.toInt()),
    editedAt: $checkedConvert('editedAt', (v) => (v as num?)?.toInt()),
    hiddenAt: $checkedConvert('hiddenAt', (v) => (v as num?)?.toInt()),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
    lastActivityAt: $checkedConvert(
      'lastActivityAt',
      (v) => (v as num).toInt(),
    ),
    solvedAnswerId: $checkedConvert('solvedAnswerId', (v) => v as String?),
    viewerVote: $checkedConvert(
      'viewerVote',
      (v) => $enumDecodeNullable(
        _$ThreadDetailViewerVoteEnumEnumMap,
        v,
        unknownValue: ThreadDetailViewerVoteEnum.unknownDefaultOpenApi,
      ),
    ),
    isBookmarked: $checkedConvert('isBookmarked', (v) => v as bool),
    myLastReadCommentId: $checkedConvert(
      'myLastReadCommentId',
      (v) => v as String?,
    ),
    mySubscriptionLevel: $checkedConvert(
      'mySubscriptionLevel',
      (v) => $enumDecodeNullable(
        _$ThreadDetailMySubscriptionLevelEnumEnumMap,
        v,
        unknownValue: ThreadDetailMySubscriptionLevelEnum.unknownDefaultOpenApi,
      ),
    ),
    poll: $checkedConvert(
      'poll',
      (v) => v == null ? null : Poll.fromJson(v as Map<String, dynamic>),
    ),
    canEdit: $checkedConvert('canEdit', (v) => v as bool),
    canDelete: $checkedConvert('canDelete', (v) => v as bool),
    canModerate: $checkedConvert('canModerate', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$ThreadDetailToJson(ThreadDetail instance) =>
    <String, dynamic>{
      'id': instance.id,
      'boardId': instance.boardId,
      'authorHandle': instance.authorHandle,
      'authorDisplayName': ?instance.authorDisplayName,
      'authorAvatar': instance.authorAvatar?.toJson(),
      'authorId': instance.authorId,
      'title': instance.title,
      'body': instance.body,
      'contentFormat': _$ContentFormatEnumMap[instance.contentFormat]!,
      'contentVersion': instance.contentVersion,
      'replyCount': instance.replyCount,
      'voteCount': instance.voteCount,
      'hotScore': instance.hotScore,
      'tags': instance.tags,
      'attachments': instance.attachments.map((e) => e.toJson()).toList(),
      'status': instance.status,
      'pinnedAt': instance.pinnedAt,
      'pinnedGlobally': instance.pinnedGlobally,
      'featuredAt': instance.featuredAt,
      'closedAt': instance.closedAt,
      'archivedAt': instance.archivedAt,
      'deletedAt': instance.deletedAt,
      'editedAt': instance.editedAt,
      'hiddenAt': instance.hiddenAt,
      'createdAt': instance.createdAt,
      'lastActivityAt': instance.lastActivityAt,
      'solvedAnswerId': instance.solvedAnswerId,
      'viewerVote': _$ThreadDetailViewerVoteEnumEnumMap[instance.viewerVote],
      'isBookmarked': instance.isBookmarked,
      'myLastReadCommentId': instance.myLastReadCommentId,
      'mySubscriptionLevel':
          _$ThreadDetailMySubscriptionLevelEnumEnumMap[instance
              .mySubscriptionLevel],
      'poll': instance.poll?.toJson(),
      'canEdit': instance.canEdit,
      'canDelete': instance.canDelete,
      'canModerate': instance.canModerate,
    };

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$ThreadDetailViewerVoteEnumEnumMap = {
  ThreadDetailViewerVoteEnum.up: 'up',
  ThreadDetailViewerVoteEnum.down: 'down',
  ThreadDetailViewerVoteEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$ThreadDetailMySubscriptionLevelEnumEnumMap = {
  ThreadDetailMySubscriptionLevelEnum.watching: 'watching',
  ThreadDetailMySubscriptionLevelEnum.tracking: 'tracking',
  ThreadDetailMySubscriptionLevelEnum.muted: 'muted',
  ThreadDetailMySubscriptionLevelEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
