//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'profile_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ProfileUpdateInput {
  /// Returns a new [ProfileUpdateInput] instance.
  ProfileUpdateInput({
    required this.displayName,

    this.school,

    required this.bio,

    required this.website,
  });

  @JsonKey(name: r'displayName', required: true, includeIfNull: true)
  final String? displayName;

  /// Optional only for rolling compatibility; current clients send the public school label.
  @JsonKey(name: r'school', required: false, includeIfNull: false)
  final String? school;

  @JsonKey(name: r'bio', required: true, includeIfNull: true)
  final String? bio;

  /// HTTPS URLs only.
  @JsonKey(name: r'website', required: true, includeIfNull: true)
  final String? website;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ProfileUpdateInput &&
          other.displayName == displayName &&
          other.school == school &&
          other.bio == bio &&
          other.website == website;

  @override
  int get hashCode =>
      (displayName == null ? 0 : displayName.hashCode) +
      school.hashCode +
      (bio == null ? 0 : bio.hashCode) +
      (website == null ? 0 : website.hashCode);

  factory ProfileUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$ProfileUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$ProfileUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
