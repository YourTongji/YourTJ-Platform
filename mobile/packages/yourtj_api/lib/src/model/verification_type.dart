//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/verification_category.dart';
import 'package:yourtj_api/src/model/verification_badge_variant.dart';
import 'package:yourtj_api/src/model/verification_icon.dart';
import 'package:json_annotation/json_annotation.dart';

part 'verification_type.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class VerificationType {
  /// Returns a new [VerificationType] instance.
  VerificationType({
    required this.id,

    required this.slug,

    required this.category,

    required this.label,

    this.description,

    required this.icon,

    required this.badgeVariant,

    required this.allowsPublicDisplay,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

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

  @JsonKey(name: r'allowsPublicDisplay', required: true, includeIfNull: false)
  final bool allowsPublicDisplay;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is VerificationType &&
          other.id == id &&
          other.slug == slug &&
          other.category == category &&
          other.label == label &&
          other.description == description &&
          other.icon == icon &&
          other.badgeVariant == badgeVariant &&
          other.allowsPublicDisplay == allowsPublicDisplay &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      slug.hashCode +
      category.hashCode +
      label.hashCode +
      (description == null ? 0 : description.hashCode) +
      icon.hashCode +
      badgeVariant.hashCode +
      allowsPublicDisplay.hashCode +
      createdAt.hashCode;

  factory VerificationType.fromJson(Map<String, dynamic> json) =>
      _$VerificationTypeFromJson(json);

  Map<String, dynamic> toJson() => _$VerificationTypeToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
