//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/email_code_purpose.dart';
import 'package:json_annotation/json_annotation.dart';

part 'email_code_verification.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class EmailCodeVerification {
  /// Returns a new [EmailCodeVerification] instance.
  EmailCodeVerification({
    required this.email,

    required this.code,

    this.purpose,

    this.handle,

    this.password,

    this.clientInstallationId,
  });

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final String email;

  @JsonKey(name: r'code', required: true, includeIfNull: false)
  final String code;

  /// Optional only for rolling compatibility. This full-login endpoint rejects appeal- and recovery-purpose codes.
  @JsonKey(
    name: r'purpose',
    required: false,
    includeIfNull: false,
    unknownEnumValue: EmailCodePurpose.unknownDefaultOpenApi,
  )
  final EmailCodePurpose? purpose;

  @JsonKey(name: r'handle', required: false, includeIfNull: false)
  final String? handle;

  @JsonKey(name: r'password', required: false, includeIfNull: false)
  final String? password;

  /// Optional first-party UUID v4. The raw value remains on the first-party client; the server stores only an account-scoped digest and replaces the prior active session for the same installation.
  @JsonKey(name: r'clientInstallationId', required: false, includeIfNull: false)
  final String? clientInstallationId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is EmailCodeVerification &&
          other.email == email &&
          other.code == code &&
          other.purpose == purpose &&
          other.handle == handle &&
          other.password == password &&
          other.clientInstallationId == clientInstallationId;

  @override
  int get hashCode =>
      email.hashCode +
      code.hashCode +
      purpose.hashCode +
      handle.hashCode +
      password.hashCode +
      clientInstallationId.hashCode;

  factory EmailCodeVerification.fromJson(Map<String, dynamic> json) =>
      _$EmailCodeVerificationFromJson(json);

  Map<String, dynamic> toJson() => _$EmailCodeVerificationToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
