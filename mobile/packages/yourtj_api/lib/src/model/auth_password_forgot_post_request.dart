//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'auth_password_forgot_post_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AuthPasswordForgotPostRequest {
  /// Returns a new [AuthPasswordForgotPostRequest] instance.
  AuthPasswordForgotPostRequest({
    required this.email,

    required this.captchaToken,
  });

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final String email;

  @JsonKey(name: r'captchaToken', required: true, includeIfNull: false)
  final String captchaToken;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AuthPasswordForgotPostRequest &&
          other.email == email &&
          other.captchaToken == captchaToken;

  @override
  int get hashCode => email.hashCode + captchaToken.hashCode;

  factory AuthPasswordForgotPostRequest.fromJson(Map<String, dynamic> json) =>
      _$AuthPasswordForgotPostRequestFromJson(json);

  Map<String, dynamic> toJson() => _$AuthPasswordForgotPostRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
