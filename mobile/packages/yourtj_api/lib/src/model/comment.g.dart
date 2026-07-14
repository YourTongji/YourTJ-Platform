// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'comment.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Comment _$CommentFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('Comment', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'threadId',
      'parentId',
      'path',
      'authorHandle',
      'authorAvatar',
      'authorId',
      'body',
      'contentFormat',
      'contentVersion',
      'attachments',
      'voteCount',
      'viewerVote',
      'isBookmarked',
      'isDeleted',
      'isHidden',
      'editedAt',
      'createdAt',
      'quotedCommentId',
      'isSolved',
      'canEdit',
      'canDelete',
      'canModerate',
    ],
  );
  final val = Comment(
    id: $checkedConvert('id', (v) => v as String),
    threadId: $checkedConvert('threadId', (v) => v as String),
    parentId: $checkedConvert('parentId', (v) => v as String?),
    path: $checkedConvert('path', (v) => v as String),
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
    body: $checkedConvert('body', (v) => v as String),
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
    attachments: $checkedConvert(
      'attachments',
      (v) => (v as List<dynamic>)
          .map((e) => ForumAttachment.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    voteCount: $checkedConvert('voteCount', (v) => (v as num).toInt()),
    viewerVote: $checkedConvert(
      'viewerVote',
      (v) => $enumDecodeNullable(
        _$CommentViewerVoteEnumEnumMap,
        v,
        unknownValue: CommentViewerVoteEnum.unknownDefaultOpenApi,
      ),
    ),
    isBookmarked: $checkedConvert('isBookmarked', (v) => v as bool),
    isDeleted: $checkedConvert('isDeleted', (v) => v as bool),
    isHidden: $checkedConvert('isHidden', (v) => v as bool),
    editedAt: $checkedConvert('editedAt', (v) => (v as num?)?.toInt()),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
    quotedCommentId: $checkedConvert('quotedCommentId', (v) => v as String?),
    isSolved: $checkedConvert('isSolved', (v) => v as bool),
    canEdit: $checkedConvert('canEdit', (v) => v as bool),
    canDelete: $checkedConvert('canDelete', (v) => v as bool),
    canModerate: $checkedConvert('canModerate', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$CommentToJson(Comment instance) => <String, dynamic>{
  'id': instance.id,
  'threadId': instance.threadId,
  'parentId': instance.parentId,
  'path': instance.path,
  'authorHandle': instance.authorHandle,
  'authorDisplayName': ?instance.authorDisplayName,
  'authorAvatar': instance.authorAvatar?.toJson(),
  'authorId': instance.authorId,
  'body': instance.body,
  'contentFormat': _$ContentFormatEnumMap[instance.contentFormat]!,
  'contentVersion': instance.contentVersion,
  'attachments': instance.attachments.map((e) => e.toJson()).toList(),
  'voteCount': instance.voteCount,
  'viewerVote': _$CommentViewerVoteEnumEnumMap[instance.viewerVote],
  'isBookmarked': instance.isBookmarked,
  'isDeleted': instance.isDeleted,
  'isHidden': instance.isHidden,
  'editedAt': instance.editedAt,
  'createdAt': instance.createdAt,
  'quotedCommentId': instance.quotedCommentId,
  'isSolved': instance.isSolved,
  'canEdit': instance.canEdit,
  'canDelete': instance.canDelete,
  'canModerate': instance.canModerate,
};

const _$ContentFormatEnumMap = {
  ContentFormat.plainV1: 'plain_v1',
  ContentFormat.markdownV1: 'markdown_v1',
  ContentFormat.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$CommentViewerVoteEnumEnumMap = {
  CommentViewerVoteEnum.up: 'up',
  CommentViewerVoteEnum.down: 'down',
  CommentViewerVoteEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
