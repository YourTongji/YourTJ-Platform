//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'password_change_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PasswordChangeInput {
  /// Returns a new [PasswordChangeInput] instance.
  PasswordChangeInput({
    required this.currentPassword,

    required this.newPassword,

    this.clientInstallationId,
  });

  @JsonKey(name: r'currentPassword', required: true, includeIfNull: false)
  final String currentPassword;

  @JsonKey(name: r'newPassword', required: true, includeIfNull: false)
  final String newPassword;

  /// Optional first-party UUID v4. The raw value remains on the first-party client; the server stores only an account-scoped digest and replaces the prior active session for the same installation.
  @JsonKey(name: r'clientInstallationId', required: false, includeIfNull: false)
  final String? clientInstallationId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PasswordChangeInput &&
          other.currentPassword == currentPassword &&
          other.newPassword == newPassword &&
          other.clientInstallationId == clientInstallationId;

  @override
  int get hashCode =>
      currentPassword.hashCode +
      newPassword.hashCode +
      clientInstallationId.hashCode;

  factory PasswordChangeInput.fromJson(Map<String, dynamic> json) =>
      _$PasswordChangeInputFromJson(json);

  Map<String, dynamic> toJson() => _$PasswordChangeInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
