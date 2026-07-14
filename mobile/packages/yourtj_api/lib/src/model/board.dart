//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'board.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Board {
  /// Returns a new [Board] instance.
  Board({
    required this.id,

    required this.slug,

    required this.name,

    required this.parentId,

    required this.description,

    required this.position,

    required this.isLocked,

    required this.minTrustToPost,

    required this.isQa,

    required this.threadCount,

    required this.canPost,

    required this.postingRestriction,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'slug', required: true, includeIfNull: false)
  final String slug;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'parentId', required: true, includeIfNull: true)
  final String? parentId;

  @JsonKey(name: r'description', required: true, includeIfNull: true)
  final String? description;

  @JsonKey(name: r'position', required: true, includeIfNull: false)
  final int position;

  @JsonKey(name: r'isLocked', required: true, includeIfNull: false)
  final bool isLocked;

  // minimum: 1
  // maximum: 6
  @JsonKey(name: r'minTrustToPost', required: true, includeIfNull: false)
  final int minTrustToPost;

  @JsonKey(name: r'isQa', required: true, includeIfNull: false)
  final bool isQa;

  // minimum: 0
  @JsonKey(name: r'threadCount', required: true, includeIfNull: false)
  final int threadCount;

  @JsonKey(name: r'canPost', required: true, includeIfNull: false)
  final bool canPost;

  @JsonKey(
    name: r'postingRestriction',
    required: true,
    includeIfNull: true,
    unknownEnumValue: BoardPostingRestrictionEnum.unknownDefaultOpenApi,
  )
  final BoardPostingRestrictionEnum? postingRestriction;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Board &&
          other.id == id &&
          other.slug == slug &&
          other.name == name &&
          other.parentId == parentId &&
          other.description == description &&
          other.position == position &&
          other.isLocked == isLocked &&
          other.minTrustToPost == minTrustToPost &&
          other.isQa == isQa &&
          other.threadCount == threadCount &&
          other.canPost == canPost &&
          other.postingRestriction == postingRestriction;

  @override
  int get hashCode =>
      id.hashCode +
      slug.hashCode +
      name.hashCode +
      (parentId == null ? 0 : parentId.hashCode) +
      (description == null ? 0 : description.hashCode) +
      position.hashCode +
      isLocked.hashCode +
      minTrustToPost.hashCode +
      isQa.hashCode +
      threadCount.hashCode +
      canPost.hashCode +
      (postingRestriction == null ? 0 : postingRestriction.hashCode);

  factory Board.fromJson(Map<String, dynamic> json) => _$BoardFromJson(json);

  Map<String, dynamic> toJson() => _$BoardToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum BoardPostingRestrictionEnum {
  @JsonValue(r'login_required')
  loginRequired(r'login_required'),
  @JsonValue(r'board_locked')
  boardLocked(r'board_locked'),
  @JsonValue(r'trust_level')
  trustLevel(r'trust_level'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const BoardPostingRestrictionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
