//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_board_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminBoardUpdateInput {
  /// Returns a new [AdminBoardUpdateInput] instance.
  AdminBoardUpdateInput({
    this.slug,

    this.name,

    this.description,

    this.position,

    this.isLocked,

    this.minTrustToPost,

    this.isQa,

    required this.reason,
  });

  @JsonKey(name: r'slug', required: false, includeIfNull: false)
  final String? slug;

  @JsonKey(name: r'name', required: false, includeIfNull: false)
  final String? name;

  @JsonKey(name: r'description', required: false, includeIfNull: false)
  final String? description;

  // minimum: 0
  @JsonKey(name: r'position', required: false, includeIfNull: false)
  final int? position;

  @JsonKey(name: r'isLocked', required: false, includeIfNull: false)
  final bool? isLocked;

  // minimum: 1
  // maximum: 6
  @JsonKey(name: r'minTrustToPost', required: false, includeIfNull: false)
  final int? minTrustToPost;

  @JsonKey(name: r'isQa', required: false, includeIfNull: false)
  final bool? isQa;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminBoardUpdateInput &&
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

  factory AdminBoardUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$AdminBoardUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminBoardUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
