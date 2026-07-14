//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/poll.dart';
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:yourtj_api/src/model/forum_attachment.dart';
import 'package:yourtj_api/src/model/media_delivery.dart';
import 'package:json_annotation/json_annotation.dart';

part 'thread_detail.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ThreadDetail {
  /// Returns a new [ThreadDetail] instance.
  ThreadDetail({
    required this.id,

    required this.boardId,

    required this.authorHandle,

    this.authorDisplayName,

    required this.authorAvatar,

    required this.authorId,

    required this.title,

    required this.body,

    required this.contentFormat,

    required this.contentVersion,

    required this.replyCount,

    required this.voteCount,

    required this.hotScore,

    required this.tags,

    required this.attachments,

    required this.status,

    required this.pinnedAt,

    required this.pinnedGlobally,

    required this.featuredAt,

    required this.closedAt,

    required this.archivedAt,

    required this.deletedAt,

    required this.editedAt,

    required this.hiddenAt,

    required this.createdAt,

    required this.lastActivityAt,

    required this.solvedAnswerId,

    required this.viewerVote,

    required this.isBookmarked,

    required this.myLastReadCommentId,

    required this.mySubscriptionLevel,

    required this.poll,

    required this.canEdit,

    required this.canDelete,

    required this.canModerate,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'boardId', required: true, includeIfNull: false)
  final String boardId;

  @JsonKey(name: r'authorHandle', required: true, includeIfNull: false)
  final String authorHandle;

  @JsonKey(name: r'authorDisplayName', required: false, includeIfNull: false)
  final String? authorDisplayName;

  /// Current short-lived clean thumb_256 avatar projection for the active author; null when no publishable avatar is available.
  @JsonKey(name: r'authorAvatar', required: true, includeIfNull: true)
  final MediaDelivery? authorAvatar;

  @JsonKey(name: r'authorId', required: true, includeIfNull: false)
  final String authorId;

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'body', required: true, includeIfNull: true)
  final String? body;

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

  @JsonKey(name: r'replyCount', required: true, includeIfNull: false)
  final int replyCount;

  @JsonKey(name: r'voteCount', required: true, includeIfNull: false)
  final int voteCount;

  @JsonKey(name: r'hotScore', required: true, includeIfNull: true)
  final num? hotScore;

  @JsonKey(name: r'tags', required: true, includeIfNull: false)
  final List<String> tags;

  @JsonKey(name: r'attachments', required: true, includeIfNull: false)
  final List<ForumAttachment> attachments;

  @JsonKey(name: r'status', required: true, includeIfNull: false)
  final String status;

  @JsonKey(name: r'pinnedAt', required: true, includeIfNull: true)
  final int? pinnedAt;

  @JsonKey(name: r'pinnedGlobally', required: true, includeIfNull: false)
  final bool pinnedGlobally;

  @JsonKey(name: r'featuredAt', required: true, includeIfNull: true)
  final int? featuredAt;

  @JsonKey(name: r'closedAt', required: true, includeIfNull: true)
  final int? closedAt;

  @JsonKey(name: r'archivedAt', required: true, includeIfNull: true)
  final int? archivedAt;

  @JsonKey(name: r'deletedAt', required: true, includeIfNull: true)
  final int? deletedAt;

  @JsonKey(name: r'editedAt', required: true, includeIfNull: true)
  final int? editedAt;

  @JsonKey(name: r'hiddenAt', required: true, includeIfNull: true)
  final int? hiddenAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'lastActivityAt', required: true, includeIfNull: false)
  final int lastActivityAt;

  @JsonKey(name: r'solvedAnswerId', required: true, includeIfNull: true)
  final String? solvedAnswerId;

  @JsonKey(
    name: r'viewerVote',
    required: true,
    includeIfNull: true,
    unknownEnumValue: ThreadDetailViewerVoteEnum.unknownDefaultOpenApi,
  )
  final ThreadDetailViewerVoteEnum? viewerVote;

  @JsonKey(name: r'isBookmarked', required: true, includeIfNull: false)
  final bool isBookmarked;

  @JsonKey(name: r'myLastReadCommentId', required: true, includeIfNull: true)
  final String? myLastReadCommentId;

  @JsonKey(
    name: r'mySubscriptionLevel',
    required: true,
    includeIfNull: true,
    unknownEnumValue: ThreadDetailMySubscriptionLevelEnum.unknownDefaultOpenApi,
  )
  final ThreadDetailMySubscriptionLevelEnum? mySubscriptionLevel;

  @JsonKey(name: r'poll', required: true, includeIfNull: true)
  final Poll? poll;

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
      other is ThreadDetail &&
          other.id == id &&
          other.boardId == boardId &&
          other.authorHandle == authorHandle &&
          other.authorDisplayName == authorDisplayName &&
          other.authorAvatar == authorAvatar &&
          other.authorId == authorId &&
          other.title == title &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.contentVersion == contentVersion &&
          other.replyCount == replyCount &&
          other.voteCount == voteCount &&
          other.hotScore == hotScore &&
          other.tags == tags &&
          other.attachments == attachments &&
          other.status == status &&
          other.pinnedAt == pinnedAt &&
          other.pinnedGlobally == pinnedGlobally &&
          other.featuredAt == featuredAt &&
          other.closedAt == closedAt &&
          other.archivedAt == archivedAt &&
          other.deletedAt == deletedAt &&
          other.editedAt == editedAt &&
          other.hiddenAt == hiddenAt &&
          other.createdAt == createdAt &&
          other.lastActivityAt == lastActivityAt &&
          other.solvedAnswerId == solvedAnswerId &&
          other.viewerVote == viewerVote &&
          other.isBookmarked == isBookmarked &&
          other.myLastReadCommentId == myLastReadCommentId &&
          other.mySubscriptionLevel == mySubscriptionLevel &&
          other.poll == poll &&
          other.canEdit == canEdit &&
          other.canDelete == canDelete &&
          other.canModerate == canModerate;

  @override
  int get hashCode =>
      id.hashCode +
      boardId.hashCode +
      authorHandle.hashCode +
      (authorDisplayName == null ? 0 : authorDisplayName.hashCode) +
      (authorAvatar == null ? 0 : authorAvatar.hashCode) +
      authorId.hashCode +
      title.hashCode +
      (body == null ? 0 : body.hashCode) +
      contentFormat.hashCode +
      contentVersion.hashCode +
      replyCount.hashCode +
      voteCount.hashCode +
      (hotScore == null ? 0 : hotScore.hashCode) +
      tags.hashCode +
      attachments.hashCode +
      status.hashCode +
      (pinnedAt == null ? 0 : pinnedAt.hashCode) +
      pinnedGlobally.hashCode +
      (featuredAt == null ? 0 : featuredAt.hashCode) +
      (closedAt == null ? 0 : closedAt.hashCode) +
      (archivedAt == null ? 0 : archivedAt.hashCode) +
      (deletedAt == null ? 0 : deletedAt.hashCode) +
      (editedAt == null ? 0 : editedAt.hashCode) +
      (hiddenAt == null ? 0 : hiddenAt.hashCode) +
      createdAt.hashCode +
      lastActivityAt.hashCode +
      (solvedAnswerId == null ? 0 : solvedAnswerId.hashCode) +
      (viewerVote == null ? 0 : viewerVote.hashCode) +
      isBookmarked.hashCode +
      (myLastReadCommentId == null ? 0 : myLastReadCommentId.hashCode) +
      (mySubscriptionLevel == null ? 0 : mySubscriptionLevel.hashCode) +
      (poll == null ? 0 : poll.hashCode) +
      canEdit.hashCode +
      canDelete.hashCode +
      canModerate.hashCode;

  factory ThreadDetail.fromJson(Map<String, dynamic> json) =>
      _$ThreadDetailFromJson(json);

  Map<String, dynamic> toJson() => _$ThreadDetailToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ThreadDetailViewerVoteEnum {
  @JsonValue(r'up')
  up(r'up'),
  @JsonValue(r'down')
  down(r'down'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ThreadDetailViewerVoteEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum ThreadDetailMySubscriptionLevelEnum {
  @JsonValue(r'watching')
  watching(r'watching'),
  @JsonValue(r'tracking')
  tracking(r'tracking'),
  @JsonValue(r'muted')
  muted(r'muted'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ThreadDetailMySubscriptionLevelEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
