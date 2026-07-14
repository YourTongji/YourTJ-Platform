//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'password_reset_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PasswordResetInput {
  /// Returns a new [PasswordResetInput] instance.
  PasswordResetInput({
    required this.email,

    required this.code,

    required this.newPassword,

    this.clientInstallationId,
  });

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final String email;

  @JsonKey(name: r'code', required: true, includeIfNull: false)
  final String code;

  @JsonKey(name: r'newPassword', required: true, includeIfNull: false)
  final String newPassword;

  /// Optional first-party UUID v4. The raw value remains on the first-party client; the server stores only an account-scoped digest and replaces the prior active session for the same installation.
  @JsonKey(name: r'clientInstallationId', required: false, includeIfNull: false)
  final String? clientInstallationId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PasswordResetInput &&
          other.email == email &&
          other.code == code &&
          other.newPassword == newPassword &&
          other.clientInstallationId == clientInstallationId;

  @override
  int get hashCode =>
      email.hashCode +
      code.hashCode +
      newPassword.hashCode +
      clientInstallationId.hashCode;

  factory PasswordResetInput.fromJson(Map<String, dynamic> json) =>
      _$PasswordResetInputFromJson(json);

  Map<String, dynamic> toJson() => _$PasswordResetInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
