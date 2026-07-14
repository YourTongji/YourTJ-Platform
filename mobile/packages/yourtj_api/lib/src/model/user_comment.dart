//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:yourtj_api/src/model/forum_attachment.dart';
import 'package:json_annotation/json_annotation.dart';

part 'user_comment.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UserComment {
  /// Returns a new [UserComment] instance.
  UserComment({
    required this.id,

    required this.threadId,

    required this.threadTitle,

    required this.body,

    required this.contentFormat,

    required this.replyCount,

    required this.voteCount,

    required this.viewerVote,

    required this.isBookmarked,

    required this.attachments,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'threadId', required: true, includeIfNull: false)
  final String threadId;

  @JsonKey(name: r'threadTitle', required: true, includeIfNull: false)
  final String threadTitle;

  @JsonKey(name: r'body', required: true, includeIfNull: false)
  final String body;

  @JsonKey(
    name: r'contentFormat',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ContentFormat.unknownDefaultOpenApi,
  )
  final ContentFormat contentFormat;

  /// Count of directly nested comments that remain visible.
  // minimum: 0
  @JsonKey(name: r'replyCount', required: true, includeIfNull: false)
  final int replyCount;

  @JsonKey(name: r'voteCount', required: true, includeIfNull: false)
  final int voteCount;

  @JsonKey(
    name: r'viewerVote',
    required: true,
    includeIfNull: true,
    unknownEnumValue: UserCommentViewerVoteEnum.unknownDefaultOpenApi,
  )
  final UserCommentViewerVoteEnum? viewerVote;

  @JsonKey(name: r'isBookmarked', required: true, includeIfNull: false)
  final bool isBookmarked;

  @JsonKey(name: r'attachments', required: true, includeIfNull: false)
  final List<ForumAttachment> attachments;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UserComment &&
          other.id == id &&
          other.threadId == threadId &&
          other.threadTitle == threadTitle &&
          other.body == body &&
          other.contentFormat == contentFormat &&
          other.replyCount == replyCount &&
          other.voteCount == voteCount &&
          other.viewerVote == viewerVote &&
          other.isBookmarked == isBookmarked &&
          other.attachments == attachments &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      threadId.hashCode +
      threadTitle.hashCode +
      body.hashCode +
      contentFormat.hashCode +
      replyCount.hashCode +
      voteCount.hashCode +
      (viewerVote == null ? 0 : viewerVote.hashCode) +
      isBookmarked.hashCode +
      attachments.hashCode +
      createdAt.hashCode;

  factory UserComment.fromJson(Map<String, dynamic> json) =>
      _$UserCommentFromJson(json);

  Map<String, dynamic> toJson() => _$UserCommentToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum UserCommentViewerVoteEnum {
  @JsonValue(r'up')
  up(r'up'),
  @JsonValue(r'down')
  down(r'down'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UserCommentViewerVoteEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
