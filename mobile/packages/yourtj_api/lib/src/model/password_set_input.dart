//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'password_set_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PasswordSetInput {
  /// Returns a new [PasswordSetInput] instance.
  PasswordSetInput({required this.newPassword, this.clientInstallationId});

  @JsonKey(name: r'newPassword', required: true, includeIfNull: false)
  final String newPassword;

  /// Optional first-party UUID v4. The raw value remains on the first-party client; the server stores only an account-scoped digest and replaces the prior active session for the same installation.
  @JsonKey(name: r'clientInstallationId', required: false, includeIfNull: false)
  final String? clientInstallationId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PasswordSetInput &&
          other.newPassword == newPassword &&
          other.clientInstallationId == clientInstallationId;

  @override
  int get hashCode => newPassword.hashCode + clientInstallationId.hashCode;

  factory PasswordSetInput.fromJson(Map<String, dynamic> json) =>
      _$PasswordSetInputFromJson(json);

  Map<String, dynamic> toJson() => _$PasswordSetInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
