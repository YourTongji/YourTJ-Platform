//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/verification_category.dart';
import 'package:yourtj_api/src/model/verification_badge_variant.dart';
import 'package:yourtj_api/src/model/verification_icon.dart';
import 'package:json_annotation/json_annotation.dart';

part 'verification_grant.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class VerificationGrant {
  /// Returns a new [VerificationGrant] instance.
  VerificationGrant({
    required this.id,

    required this.accountId,

    required this.verificationTypeId,

    required this.slug,

    required this.category,

    required this.label,

    required this.icon,

    required this.badgeVariant,

    required this.displayOnProfile,

    required this.status,

    required this.issuedBy,

    required this.issuedAt,

    this.expiresAt,

    required this.issueReason,

    required this.hasEvidence,

    this.revokedBy,

    this.revokedAt,

    this.revokeReason,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(name: r'verificationTypeId', required: true, includeIfNull: false)
  final String verificationTypeId;

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

  @JsonKey(name: r'displayOnProfile', required: true, includeIfNull: false)
  final bool displayOnProfile;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: VerificationGrantStatusEnum.unknownDefaultOpenApi,
  )
  final VerificationGrantStatusEnum status;

  @JsonKey(name: r'issuedBy', required: true, includeIfNull: true)
  final String? issuedBy;

  @JsonKey(name: r'issuedAt', required: true, includeIfNull: false)
  final int issuedAt;

  @JsonKey(name: r'expiresAt', required: false, includeIfNull: false)
  final int? expiresAt;

  @JsonKey(name: r'issueReason', required: true, includeIfNull: false)
  final String issueReason;

  @JsonKey(name: r'hasEvidence', required: true, includeIfNull: false)
  final bool hasEvidence;

  @JsonKey(name: r'revokedBy', required: false, includeIfNull: false)
  final String? revokedBy;

  @JsonKey(name: r'revokedAt', required: false, includeIfNull: false)
  final int? revokedAt;

  @JsonKey(name: r'revokeReason', required: false, includeIfNull: false)
  final String? revokeReason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is VerificationGrant &&
          other.id == id &&
          other.accountId == accountId &&
          other.verificationTypeId == verificationTypeId &&
          other.slug == slug &&
          other.category == category &&
          other.label == label &&
          other.icon == icon &&
          other.badgeVariant == badgeVariant &&
          other.displayOnProfile == displayOnProfile &&
          other.status == status &&
          other.issuedBy == issuedBy &&
          other.issuedAt == issuedAt &&
          other.expiresAt == expiresAt &&
          other.issueReason == issueReason &&
          other.hasEvidence == hasEvidence &&
          other.revokedBy == revokedBy &&
          other.revokedAt == revokedAt &&
          other.revokeReason == revokeReason;

  @override
  int get hashCode =>
      id.hashCode +
      accountId.hashCode +
      verificationTypeId.hashCode +
      slug.hashCode +
      category.hashCode +
      label.hashCode +
      icon.hashCode +
      badgeVariant.hashCode +
      displayOnProfile.hashCode +
      status.hashCode +
      (issuedBy == null ? 0 : issuedBy.hashCode) +
      issuedAt.hashCode +
      (expiresAt == null ? 0 : expiresAt.hashCode) +
      issueReason.hashCode +
      hasEvidence.hashCode +
      (revokedBy == null ? 0 : revokedBy.hashCode) +
      (revokedAt == null ? 0 : revokedAt.hashCode) +
      (revokeReason == null ? 0 : revokeReason.hashCode);

  factory VerificationGrant.fromJson(Map<String, dynamic> json) =>
      _$VerificationGrantFromJson(json);

  Map<String, dynamic> toJson() => _$VerificationGrantToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum VerificationGrantStatusEnum {
  @JsonValue(r'active')
  active(r'active'),
  @JsonValue(r'expired')
  expired(r'expired'),
  @JsonValue(r'revoked')
  revoked(r'revoked'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const VerificationGrantStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
