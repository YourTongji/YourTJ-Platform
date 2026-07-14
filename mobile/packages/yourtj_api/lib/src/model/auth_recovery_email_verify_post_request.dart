//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'auth_recovery_email_verify_post_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AuthRecoveryEmailVerifyPostRequest {
  /// Returns a new [AuthRecoveryEmailVerifyPostRequest] instance.
  AuthRecoveryEmailVerifyPostRequest({required this.email, required this.code});

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final String email;

  @JsonKey(name: r'code', required: true, includeIfNull: false)
  final String code;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AuthRecoveryEmailVerifyPostRequest &&
          other.email == email &&
          other.code == code;

  @override
  int get hashCode => email.hashCode + code.hashCode;

  factory AuthRecoveryEmailVerifyPostRequest.fromJson(
    Map<String, dynamic> json,
  ) => _$AuthRecoveryEmailVerifyPostRequestFromJson(json);

  Map<String, dynamic> toJson() =>
      _$AuthRecoveryEmailVerifyPostRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
