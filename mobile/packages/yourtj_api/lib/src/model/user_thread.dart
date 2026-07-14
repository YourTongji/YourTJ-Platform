//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/content_format.dart';
import 'package:yourtj_api/src/model/forum_attachment.dart';
import 'package:json_annotation/json_annotation.dart';

part 'user_thread.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UserThread {
  /// Returns a new [UserThread] instance.
  UserThread({
    required this.id,

    required this.title,

    required this.bodyExcerpt,

    required this.contentFormat,

    required this.boardSlug,

    required this.replyCount,

    required this.voteCount,

    required this.viewerVote,

    required this.isBookmarked,

    required this.attachments,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'bodyExcerpt', required: true, includeIfNull: true)
  final String? bodyExcerpt;

  @JsonKey(
    name: r'contentFormat',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ContentFormat.unknownDefaultOpenApi,
  )
  final ContentFormat contentFormat;

  @JsonKey(name: r'boardSlug', required: true, includeIfNull: false)
  final String boardSlug;

  // minimum: 0
  @JsonKey(name: r'replyCount', required: true, includeIfNull: false)
  final int replyCount;

  @JsonKey(name: r'voteCount', required: true, includeIfNull: false)
  final int voteCount;

  @JsonKey(
    name: r'viewerVote',
    required: true,
    includeIfNull: true,
    unknownEnumValue: UserThreadViewerVoteEnum.unknownDefaultOpenApi,
  )
  final UserThreadViewerVoteEnum? viewerVote;

  @JsonKey(name: r'isBookmarked', required: true, includeIfNull: false)
  final bool isBookmarked;

  @JsonKey(name: r'attachments', required: true, includeIfNull: false)
  final List<ForumAttachment> attachments;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UserThread &&
          other.id == id &&
          other.title == title &&
          other.bodyExcerpt == bodyExcerpt &&
          other.contentFormat == contentFormat &&
          other.boardSlug == boardSlug &&
          other.replyCount == replyCount &&
          other.voteCount == voteCount &&
          other.viewerVote == viewerVote &&
          other.isBookmarked == isBookmarked &&
          other.attachments == attachments &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      title.hashCode +
      (bodyExcerpt == null ? 0 : bodyExcerpt.hashCode) +
      contentFormat.hashCode +
      boardSlug.hashCode +
      replyCount.hashCode +
      voteCount.hashCode +
      (viewerVote == null ? 0 : viewerVote.hashCode) +
      isBookmarked.hashCode +
      attachments.hashCode +
      createdAt.hashCode;

  factory UserThread.fromJson(Map<String, dynamic> json) =>
      _$UserThreadFromJson(json);

  Map<String, dynamic> toJson() => _$UserThreadToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum UserThreadViewerVoteEnum {
  @JsonValue(r'up')
  up(r'up'),
  @JsonValue(r'down')
  down(r'down'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UserThreadViewerVoteEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
