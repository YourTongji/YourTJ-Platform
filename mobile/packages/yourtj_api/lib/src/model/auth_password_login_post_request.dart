//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'auth_password_login_post_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AuthPasswordLoginPostRequest {
  /// Returns a new [AuthPasswordLoginPostRequest] instance.
  AuthPasswordLoginPostRequest({
    required this.email,

    required this.password,

    this.clientInstallationId,
  });

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final String email;

  @JsonKey(name: r'password', required: true, includeIfNull: false)
  final String password;

  /// Optional first-party UUID v4. The raw value remains on the first-party client; the server stores only an account-scoped digest and replaces the prior active session for the same installation.
  @JsonKey(name: r'clientInstallationId', required: false, includeIfNull: false)
  final String? clientInstallationId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AuthPasswordLoginPostRequest &&
          other.email == email &&
          other.password == password &&
          other.clientInstallationId == clientInstallationId;

  @override
  int get hashCode =>
      email.hashCode + password.hashCode + clientInstallationId.hashCode;

  factory AuthPasswordLoginPostRequest.fromJson(Map<String, dynamic> json) =>
      _$AuthPasswordLoginPostRequestFromJson(json);

  Map<String, dynamic> toJson() => _$AuthPasswordLoginPostRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
