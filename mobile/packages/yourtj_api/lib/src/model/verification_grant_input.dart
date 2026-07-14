//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'verification_grant_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class VerificationGrantInput {
  /// Returns a new [VerificationGrantInput] instance.
  VerificationGrantInput({
    required this.verificationTypeId,

    this.displayOnProfile = false,

    this.expiresAt,

    this.evidenceReference,

    required this.reason,
  });

  @JsonKey(name: r'verificationTypeId', required: true, includeIfNull: false)
  final String verificationTypeId;

  @JsonKey(
    defaultValue: false,
    name: r'displayOnProfile',
    required: true,
    includeIfNull: false,
  )
  final bool displayOnProfile;

  @JsonKey(name: r'expiresAt', required: false, includeIfNull: false)
  final int? expiresAt;

  /// Private opaque pointer to separately governed evidence; never returned by public or staff list responses.
  @JsonKey(name: r'evidenceReference', required: false, includeIfNull: false)
  final String? evidenceReference;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is VerificationGrantInput &&
          other.verificationTypeId == verificationTypeId &&
          other.displayOnProfile == displayOnProfile &&
          other.expiresAt == expiresAt &&
          other.evidenceReference == evidenceReference &&
          other.reason == reason;

  @override
  int get hashCode =>
      verificationTypeId.hashCode +
      displayOnProfile.hashCode +
      (expiresAt == null ? 0 : expiresAt.hashCode) +
      (evidenceReference == null ? 0 : evidenceReference.hashCode) +
      reason.hashCode;

  factory VerificationGrantInput.fromJson(Map<String, dynamic> json) =>
      _$VerificationGrantInputFromJson(json);

  Map<String, dynamic> toJson() => _$VerificationGrantInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
