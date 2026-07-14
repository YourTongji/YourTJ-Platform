//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/verification_category.dart';
import 'package:yourtj_api/src/model/verification_badge_variant.dart';
import 'package:yourtj_api/src/model/verification_icon.dart';
import 'package:json_annotation/json_annotation.dart';

part 'verification_type_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class VerificationTypeInput {
  /// Returns a new [VerificationTypeInput] instance.
  VerificationTypeInput({
    required this.slug,

    required this.category,

    required this.label,

    this.description,

    required this.icon,

    required this.badgeVariant,

    this.allowsPublicDisplay = false,

    required this.reason,
  });

  @JsonKey(name: r'slug', required: true, includeIfNull: false)
  final String slug;

  @JsonKey(
    name: r'category',
    required: true,
    includeIfNull: false,
    unknownEnumValue: VerificationCategory.unknownDefaultOpenApi,
  )
  final VerificationCategory category;

  @JsonKey(name: r'label', required: true, includeIfNull: false)
  final String label;

  @JsonKey(name: r'description', required: false, includeIfNull: false)
  final String? description;

  @JsonKey(
    name: r'icon',
    required: true,
    includeIfNull: false,
    unknownEnumValue: VerificationIcon.unknownDefaultOpenApi,
  )
  final VerificationIcon icon;

  @JsonKey(
    name: r'badgeVariant',
    required: true,
    includeIfNull: false,
    unknownEnumValue: VerificationBadgeVariant.unknownDefaultOpenApi,
  )
  final VerificationBadgeVariant badgeVariant;

  @JsonKey(
    defaultValue: false,
    name: r'allowsPublicDisplay',
    required: true,
    includeIfNull: false,
  )
  final bool allowsPublicDisplay;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is VerificationTypeInput &&
          other.slug == slug &&
          other.category == category &&
          other.label == label &&
          other.description == description &&
          other.icon == icon &&
          other.badgeVariant == badgeVariant &&
          other.allowsPublicDisplay == allowsPublicDisplay &&
          other.reason == reason;

  @override
  int get hashCode =>
      slug.hashCode +
      category.hashCode +
      label.hashCode +
      (description == null ? 0 : description.hashCode) +
      icon.hashCode +
      badgeVariant.hashCode +
      allowsPublicDisplay.hashCode +
      reason.hashCode;

  factory VerificationTypeInput.fromJson(Map<String, dynamic> json) =>
      _$VerificationTypeInputFromJson(json);

  Map<String, dynamic> toJson() => _$VerificationTypeInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
