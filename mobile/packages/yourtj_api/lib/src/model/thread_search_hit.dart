//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'thread_search_hit.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ThreadSearchHit {
  /// Returns a new [ThreadSearchHit] instance.
  ThreadSearchHit({
    required this.id,

    required this.title,

    required this.bodyExcerpt,

    required this.board,

    required this.tags,

    required this.authorHandle,

    required this.replyCount,

    required this.voteCount,

    required this.createdAt,

    required this.status,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'title', required: true, includeIfNull: false)
  final String title;

  @JsonKey(name: r'bodyExcerpt', required: true, includeIfNull: false)
  final String bodyExcerpt;

  @JsonKey(name: r'board', required: true, includeIfNull: false)
  final String board;

  @JsonKey(name: r'tags', required: true, includeIfNull: false)
  final List<String> tags;

  @JsonKey(name: r'authorHandle', required: true, includeIfNull: false)
  final String authorHandle;

  // minimum: 0
  @JsonKey(name: r'replyCount', required: true, includeIfNull: false)
  final int replyCount;

  @JsonKey(name: r'voteCount', required: true, includeIfNull: false)
  final int voteCount;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ThreadSearchHitStatusEnum.unknownDefaultOpenApi,
  )
  final ThreadSearchHitStatusEnum status;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ThreadSearchHit &&
          other.id == id &&
          other.title == title &&
          other.bodyExcerpt == bodyExcerpt &&
          other.board == board &&
          other.tags == tags &&
          other.authorHandle == authorHandle &&
          other.replyCount == replyCount &&
          other.voteCount == voteCount &&
          other.createdAt == createdAt &&
          other.status == status;

  @override
  int get hashCode =>
      id.hashCode +
      title.hashCode +
      bodyExcerpt.hashCode +
      board.hashCode +
      tags.hashCode +
      authorHandle.hashCode +
      replyCount.hashCode +
      voteCount.hashCode +
      createdAt.hashCode +
      status.hashCode;

  factory ThreadSearchHit.fromJson(Map<String, dynamic> json) =>
      _$ThreadSearchHitFromJson(json);

  Map<String, dynamic> toJson() => _$ThreadSearchHitToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ThreadSearchHitStatusEnum {
  @JsonValue(r'visible')
  visible(r'visible'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ThreadSearchHitStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
