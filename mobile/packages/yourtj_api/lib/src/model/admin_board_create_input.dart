//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_board_create_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminBoardCreateInput {
  /// Returns a new [AdminBoardCreateInput] instance.
  AdminBoardCreateInput({
    required this.slug,

    required this.name,

    this.description,

    this.position = 0,

    this.isLocked = false,

    this.minTrustToPost = 1,

    this.isQa = false,

    required this.reason,
  });

  @JsonKey(name: r'slug', required: true, includeIfNull: false)
  final String slug;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'description', required: false, includeIfNull: false)
  final String? description;

  // minimum: 0
  @JsonKey(
    defaultValue: 0,
    name: r'position',
    required: false,
    includeIfNull: false,
  )
  final int? position;

  @JsonKey(
    defaultValue: false,
    name: r'isLocked',
    required: false,
    includeIfNull: false,
  )
  final bool? isLocked;

  // minimum: 1
  // maximum: 6
  @JsonKey(
    defaultValue: 1,
    name: r'minTrustToPost',
    required: false,
    includeIfNull: false,
  )
  final int? minTrustToPost;

  @JsonKey(
    defaultValue: false,
    name: r'isQa',
    required: false,
    includeIfNull: false,
  )
  final bool? isQa;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminBoardCreateInput &&
          other.slug == slug &&
          other.name == name &&
          other.description == description &&
          other.position == position &&
          other.isLocked == isLocked &&
          other.minTrustToPost == minTrustToPost &&
          other.isQa == isQa &&
          other.reason == reason;

  @override
  int get hashCode =>
      slug.hashCode +
      name.hashCode +
      description.hashCode +
      position.hashCode +
      isLocked.hashCode +
      minTrustToPost.hashCode +
      isQa.hashCode +
      reason.hashCode;

  factory AdminBoardCreateInput.fromJson(Map<String, dynamic> json) =>
      _$AdminBoardCreateInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminBoardCreateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
