//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_tag_create_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminTagCreateInput {
  /// Returns a new [AdminTagCreateInput] instance.
  AdminTagCreateInput({
    required this.slug,

    required this.name,

    this.description,

    required this.reason,
  });

  @JsonKey(name: r'slug', required: true, includeIfNull: false)
  final String slug;

  @JsonKey(name: r'name', required: true, includeIfNull: false)
  final String name;

  @JsonKey(name: r'description', required: false, includeIfNull: false)
  final String? description;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminTagCreateInput &&
          other.slug == slug &&
          other.name == name &&
          other.description == description &&
          other.reason == reason;

  @override
  int get hashCode =>
      slug.hashCode + name.hashCode + description.hashCode + reason.hashCode;

  factory AdminTagCreateInput.fromJson(Map<String, dynamic> json) =>
      _$AdminTagCreateInputFromJson(json);

  Map<String, dynamic> toJson() => _$AdminTagCreateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
