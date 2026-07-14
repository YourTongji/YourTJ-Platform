//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

/// Optional intended image surface persisted across moderation and page reloads; it is not a business binding.
enum MediaUsage {
  /// Optional intended image surface persisted across moderation and page reloads; it is not a business binding.
  @JsonValue(r'profile_avatar')
  profileAvatar(r'profile_avatar'),

  /// Optional intended image surface persisted across moderation and page reloads; it is not a business binding.
  @JsonValue(r'profile_banner')
  profileBanner(r'profile_banner'),

  /// Optional intended image surface persisted across moderation and page reloads; it is not a business binding.
  @JsonValue(r'forum_thread')
  forumThread(r'forum_thread'),

  /// Optional intended image surface persisted across moderation and page reloads; it is not a business binding.
  @JsonValue(r'forum_comment')
  forumComment(r'forum_comment'),

  /// Optional intended image surface persisted across moderation and page reloads; it is not a business binding.
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const MediaUsage(this.value);

  final String value;

  @override
  String toString() => value;
}
