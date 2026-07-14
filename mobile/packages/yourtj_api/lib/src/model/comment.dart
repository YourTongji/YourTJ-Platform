//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:yourtj_api/src/model/forum_attachment.dart';
import 'package:yourtj_api/src/model/media_delivery.dart';
import 'package:json_annotation/json_annotation.dart';

part 'comment.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Comment {
  /// Returns a new [Comment] instance.
  Comment({
    required this.id,

    required this.threadId,

    required this.parentId,

    required this.path,

    required this.authorHandle,

    this.authorDisplayName,

    required this.authorAvatar,

    required this.authorId,

    required this.body,

    required this.contentFormat,

    required this.contentVersion,

    required this.attachments,

    required this.voteCount,

    required this.viewerVote,

    required this.isBookmarked,

    required this.isDeleted,

    required this.isHidden,

    required this.editedAt,

    required this.createdAt,

    required this.quotedCommentId,

    required this.isSolved,

    required this.canEdit,

    required this.canDelete,

    required this.canModerate,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'threadId', required: true, includeIfNull: false)
  final String threadId;

  @JsonKey(name: r'parentId', required: true, includeIfNull: true)
  final String? parentId;

  @JsonKey(name: r'path', required: true, includeIfNull: false)
  final String path;

  @JsonKey(name: r'authorHandle', required: true, includeIfNull: false)
  final String authorHandle;

  @JsonKey(name: r'authorDisplayName', required: false, includeIfNull: false)
  final String? authorDisplayName;

  /// Current short-lived clean thumb_256 avatar projection for the active author; null when no publishable avatar is available.
  @JsonKey(name: r'authorAvatar', required: true, includeIfNull: true)
  final MediaDelivery? authorAvatar;

  @JsonKey(name: r'authorId', required: true, includeIfNull: false)
  final String authorId;

  @JsonKey(name: r'body', required: true, includeIfNull: false)
  final String body;

  @JsonKey(
    name: r'contentFormat',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ContentFormat.unknownDefaultOpenApi,
  )
  final ContentFormat contentFormat;

  // minimum: 1
  @JsonKey(name: r'contentVersion', required: true, includeIfNull: false)
  final int contentVersion;

  @JsonKey(name: r'attachments', required: true, includeIfNull: false)
  final List<ForumAttachment> attachments;

  @JsonKey(name: r'voteCount', required: true, includeIfNull: false)
  final int voteCount;

  @JsonKey(
    name: r'viewerVote',
    required: true,
    includeIfNull: true,
    unknownEnumValue: CommentViewerVoteEnum.unknownDefaultOpenApi,
  )
  final CommentViewerVoteEnum? viewerVote;

  @JsonKey(name: r'isBookmarked', required: true, includeIfNull: false)
  final bool isBookmarked;

  @JsonKey(name: r'isDeleted', required: true, includeIfNull: false)
  final bool isDeleted;

  @JsonKey(name: r'isHidden', required: true, includeIfNull: false)
  final bool isHidden;

  @JsonKey(name: r'editedAt', required: true, includeIfNull: true)
  final int? editedAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'quotedCommentId', required: true, includeIfNull: true)
  final String? quotedCommentId;

  @JsonKey(name: r'isSolved', required: true, includeIfNull: false)
  final bool isSolved;

  /// Server-authoritative author edit permission for this viewer.
  @JsonKey(name: r'canEdit', required: true, includeIfNull: false)
  final bool canEdit;

  /// Server-authoritative author delete permission for this viewer.
  @JsonKey(name: r'canDelete', required: true, includeIfNull: false)
  final bool canDelete;

  /// Server-authoritative moderation permission including role hierarchy.
  @JsonKey(name: r'canModerate', required: true, includeIfNull: false)
  final bool canModerate;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Comment &&
          other.id == id &&
          other.threadId == threadId &&
          other.parentId == parentId &&
          other.path == path &&
          other.authorHandle == authorHandle &&
          other.authorDisplayName == authorDisplayName &&
          other.authorAvatar == authorAvatar &&
          other.authorId == authorId &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.contentVersion == contentVersion &&
          other.attachments == attachments &&
          other.voteCount == voteCount &&
          other.viewerVote == viewerVote &&
          other.isBookmarked == isBookmarked &&
          other.isDeleted == isDeleted &&
          other.isHidden == isHidden &&
          other.editedAt == editedAt &&
          other.createdAt == createdAt &&
          other.quotedCommentId == quotedCommentId &&
          other.isSolved == isSolved &&
          other.canEdit == canEdit &&
          other.canDelete == canDelete &&
          other.canModerate == canModerate;

  @override
  int get hashCode =>
      id.hashCode +
      threadId.hashCode +
      (parentId == null ? 0 : parentId.hashCode) +
      path.hashCode +
      authorHandle.hashCode +
      (authorDisplayName == null ? 0 : authorDisplayName.hashCode) +
      (authorAvatar == null ? 0 : authorAvatar.hashCode) +
      authorId.hashCode +
      body.hashCode +
      contentFormat.hashCode +
      contentVersion.hashCode +
      attachments.hashCode +
      voteCount.hashCode +
      (viewerVote == null ? 0 : viewerVote.hashCode) +
      isBookmarked.hashCode +
      isDeleted.hashCode +
      isHidden.hashCode +
      (editedAt == null ? 0 : editedAt.hashCode) +
      createdAt.hashCode +
      (quotedCommentId == null ? 0 : quotedCommentId.hashCode) +
      isSolved.hashCode +
      canEdit.hashCode +
      canDelete.hashCode +
      canModerate.hashCode;

  factory Comment.fromJson(Map<String, dynamic> json) =>
      _$CommentFromJson(json);

  Map<String, dynamic> toJson() => _$CommentToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum CommentViewerVoteEnum {
  @JsonValue(r'up')
  up(r'up'),
  @JsonValue(r'down')
  down(r'down'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const CommentViewerVoteEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
