//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'appeal_email_verification.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AppealEmailVerification {
  /// Returns a new [AppealEmailVerification] instance.
  AppealEmailVerification({required this.email, required this.code});

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final String email;

  @JsonKey(name: r'code', required: true, includeIfNull: false)
  final String code;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AppealEmailVerification &&
          other.email == email &&
          other.code == code;

  @override
  int get hashCode => email.hashCode + code.hashCode;

  factory AppealEmailVerification.fromJson(Map<String, dynamic> json) =>
      _$AppealEmailVerificationFromJson(json);

  Map<String, dynamic> toJson() => _$AppealEmailVerificationToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
