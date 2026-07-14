//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:yourtj_api/src/model/forum_attachment.dart';
import 'package:json_annotation/json_annotation.dart';

part 'profile_content.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ProfileContent {
  /// Returns a new [ProfileContent] instance.
  ProfileContent({
    required this.targetType,

    required this.id,

    required this.threadId,

    required this.title,

    required this.body,

    required this.contentFormat,

    required this.boardSlug,

    required this.authorHandle,

    required this.authorDisplayName,

    required this.replyCount,

    required this.voteCount,

    required this.viewerVote,

    required this.isBookmarked,

    required this.attachments,

    required this.createdAt,

    required this.activityAt,
  });

  @JsonKey(
    name: r'targetType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ProfileContentTargetTypeEnum.unknownDefaultOpenApi,
  )
  final ProfileContentTargetTypeEnum targetType;

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'threadId', required: true, includeIfNull: false)
  final String threadId;

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

  @JsonKey(name: r'boardSlug', required: true, includeIfNull: false)
  final String boardSlug;

  @JsonKey(name: r'authorHandle', required: true, includeIfNull: false)
  final String authorHandle;

  @JsonKey(name: r'authorDisplayName', required: true, includeIfNull: true)
  final String? authorDisplayName;

  // minimum: 0
  @JsonKey(name: r'replyCount', required: true, includeIfNull: false)
  final int replyCount;

  @JsonKey(name: r'voteCount', required: true, includeIfNull: false)
  final int voteCount;

  @JsonKey(
    name: r'viewerVote',
    required: true,
    includeIfNull: true,
    unknownEnumValue: ProfileContentViewerVoteEnum.unknownDefaultOpenApi,
  )
  final ProfileContentViewerVoteEnum? viewerVote;

  @JsonKey(name: r'isBookmarked', required: true, includeIfNull: false)
  final bool isBookmarked;

  @JsonKey(name: r'attachments', required: true, includeIfNull: false)
  final List<ForumAttachment> attachments;

  /// Canonical content creation time.
  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  /// Like time for liked content; otherwise the content creation time.
  @JsonKey(name: r'activityAt', required: true, includeIfNull: false)
  final int activityAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ProfileContent &&
          other.targetType == targetType &&
          other.id == id &&
          other.threadId == threadId &&
          other.title == title &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.boardSlug == boardSlug &&
          other.authorHandle == authorHandle &&
          other.authorDisplayName == authorDisplayName &&
          other.replyCount == replyCount &&
          other.voteCount == voteCount &&
          other.viewerVote == viewerVote &&
          other.isBookmarked == isBookmarked &&
          other.attachments == attachments &&
          other.createdAt == createdAt &&
          other.activityAt == activityAt;

  @override
  int get hashCode =>
      targetType.hashCode +
      id.hashCode +
      threadId.hashCode +
      title.hashCode +
      (body == null ? 0 : body.hashCode) +
      contentFormat.hashCode +
      boardSlug.hashCode +
      authorHandle.hashCode +
      (authorDisplayName == null ? 0 : authorDisplayName.hashCode) +
      replyCount.hashCode +
      voteCount.hashCode +
      (viewerVote == null ? 0 : viewerVote.hashCode) +
      isBookmarked.hashCode +
      attachments.hashCode +
      createdAt.hashCode +
      activityAt.hashCode;

  factory ProfileContent.fromJson(Map<String, dynamic> json) =>
      _$ProfileContentFromJson(json);

  Map<String, dynamic> toJson() => _$ProfileContentToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ProfileContentTargetTypeEnum {
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'comment')
  comment(r'comment'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ProfileContentTargetTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum ProfileContentViewerVoteEnum {
  @JsonValue(r'up')
  up(r'up'),
  @JsonValue(r'down')
  down(r'down'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ProfileContentViewerVoteEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
