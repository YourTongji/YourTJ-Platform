//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/profile_content.dart';
import 'package:json_annotation/json_annotation.dart';

part 'bookmark.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Bookmark {
  /// Returns a new [Bookmark] instance.
  Bookmark({
    required this.targetType,

    required this.targetId,

    required this.note,

    required this.createdAt,

    required this.content,
  });

  @JsonKey(
    name: r'targetType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: BookmarkTargetTypeEnum.unknownDefaultOpenApi,
  )
  final BookmarkTargetTypeEnum targetType;

  @JsonKey(name: r'targetId', required: true, includeIfNull: false)
  final String targetId;

  @JsonKey(name: r'note', required: true, includeIfNull: true)
  final String? note;

  /// Unix timestamp when the current account bookmarked the content.
  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'content', required: true, includeIfNull: false)
  final ProfileContent content;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Bookmark &&
          other.targetType == targetType &&
          other.targetId == targetId &&
          other.note == note &&
          other.createdAt == createdAt &&
          other.content == content;

  @override
  int get hashCode =>
      targetType.hashCode +
      targetId.hashCode +
      (note == null ? 0 : note.hashCode) +
      createdAt.hashCode +
      content.hashCode;

  factory Bookmark.fromJson(Map<String, dynamic> json) =>
      _$BookmarkFromJson(json);

  Map<String, dynamic> toJson() => _$BookmarkToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum BookmarkTargetTypeEnum {
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'comment')
  comment(r'comment'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const BookmarkTargetTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
