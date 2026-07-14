//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/forum_attachment.dart';
import 'package:yourtj_api/src/model/media_delivery.dart';
import 'package:json_annotation/json_annotation.dart';

part 'thread_feed.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ThreadFeed {
  /// Returns a new [ThreadFeed] instance.
  ThreadFeed({
    required this.id,

    required this.boardId,

    required this.authorHandle,

    this.authorDisplayName,

    required this.authorAvatar,

    required this.title,

    required this.bodyExcerpt,

    required this.contentVersion,

    required this.replyCount,

    required this.voteCount,

    required this.hotScore,

    required this.status,

    required this.createdAt,

    required this.lastActivityAt,

    required this.tags,

    required this.attachments,

    required this.viewerVote,

    required this.isBookmarked,

    required this.canEdit,

    required this.canDelete,

    required this.canModerate,

    this.unreadCount,
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

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'bodyExcerpt', required: true, includeIfNull: true)
  final String? bodyExcerpt;

  // minimum: 1
  @JsonKey(name: r'contentVersion', required: true, includeIfNull: false)
  final int contentVersion;

  @JsonKey(name: r'replyCount', required: true, includeIfNull: false)
  final int replyCount;

  @JsonKey(name: r'voteCount', required: true, includeIfNull: false)
  final int voteCount;

  @JsonKey(name: r'hotScore', required: true, includeIfNull: true)
  final num? hotScore;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ThreadFeedStatusEnum.unknownDefaultOpenApi,
  )
  final ThreadFeedStatusEnum status;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'lastActivityAt', required: true, includeIfNull: false)
  final int lastActivityAt;

  @JsonKey(name: r'tags', required: true, includeIfNull: false)
  final List<String> tags;

  /// At most the first clean image for a bounded feed card.
  @JsonKey(name: r'attachments', required: true, includeIfNull: false)
  final List<ForumAttachment> attachments;

  @JsonKey(
    name: r'viewerVote',
    required: true,
    includeIfNull: true,
    unknownEnumValue: ThreadFeedViewerVoteEnum.unknownDefaultOpenApi,
  )
  final ThreadFeedViewerVoteEnum? viewerVote;

  @JsonKey(name: r'isBookmarked', required: true, includeIfNull: false)
  final bool isBookmarked;

  /// Server-authoritative author edit permission for this viewer.
  @JsonKey(name: r'canEdit', required: true, includeIfNull: false)
  final bool canEdit;

  /// Server-authoritative author delete permission for this viewer.
  @JsonKey(name: r'canDelete', required: true, includeIfNull: false)
  final bool canDelete;

  /// Server-authoritative moderation permission including role hierarchy.
  @JsonKey(name: r'canModerate', required: true, includeIfNull: false)
  final bool canModerate;

  @JsonKey(name: r'unreadCount', required: false, includeIfNull: false)
  final int? unreadCount;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ThreadFeed &&
          other.id == id &&
          other.boardId == boardId &&
          other.authorHandle == authorHandle &&
          other.authorDisplayName == authorDisplayName &&
          other.authorAvatar == authorAvatar &&
          other.title == title &&
          other.bodyExcerpt == bodyExcerpt &&
          other.contentVersion == contentVersion &&
          other.replyCount == replyCount &&
          other.voteCount == voteCount &&
          other.hotScore == hotScore &&
          other.status == status &&
          other.createdAt == createdAt &&
          other.lastActivityAt == lastActivityAt &&
          other.tags == tags &&
          other.attachments == attachments &&
          other.viewerVote == viewerVote &&
          other.isBookmarked == isBookmarked &&
          other.canEdit == canEdit &&
          other.canDelete == canDelete &&
          other.canModerate == canModerate &&
          other.unreadCount == unreadCount;

  @override
  int get hashCode =>
      id.hashCode +
      boardId.hashCode +
      authorHandle.hashCode +
      (authorDisplayName == null ? 0 : authorDisplayName.hashCode) +
      (authorAvatar == null ? 0 : authorAvatar.hashCode) +
      title.hashCode +
      (bodyExcerpt == null ? 0 : bodyExcerpt.hashCode) +
      contentVersion.hashCode +
      replyCount.hashCode +
      voteCount.hashCode +
      (hotScore == null ? 0 : hotScore.hashCode) +
      status.hashCode +
      createdAt.hashCode +
      lastActivityAt.hashCode +
      tags.hashCode +
      attachments.hashCode +
      (viewerVote == null ? 0 : viewerVote.hashCode) +
      isBookmarked.hashCode +
      canEdit.hashCode +
      canDelete.hashCode +
      canModerate.hashCode +
      (unreadCount == null ? 0 : unreadCount.hashCode);

  factory ThreadFeed.fromJson(Map<String, dynamic> json) =>
      _$ThreadFeedFromJson(json);

  Map<String, dynamic> toJson() => _$ThreadFeedToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ThreadFeedStatusEnum {
  @JsonValue(r'visible')
  visible(r'visible'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ThreadFeedStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum ThreadFeedViewerVoteEnum {
  @JsonValue(r'up')
  up(r'up'),
  @JsonValue(r'down')
  down(r'down'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ThreadFeedViewerVoteEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
