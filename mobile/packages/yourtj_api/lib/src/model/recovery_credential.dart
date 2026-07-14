//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/account_lifecycle.dart';
import 'package:json_annotation/json_annotation.dart';

part 'recovery_credential.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class RecoveryCredential {
  /// Returns a new [RecoveryCredential] instance.
  RecoveryCredential({
    required this.recoveryToken,

    required this.expiresAt,

    required this.lifecycle,
  });

  @JsonKey(name: r'recoveryToken', required: true, includeIfNull: false)
  final String recoveryToken;

  @JsonKey(name: r'expiresAt', required: true, includeIfNull: false)
  final int expiresAt;

  @JsonKey(name: r'lifecycle', required: true, includeIfNull: false)
  final AccountLifecycle lifecycle;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is RecoveryCredential &&
          other.recoveryToken == recoveryToken &&
          other.expiresAt == expiresAt &&
          other.lifecycle == lifecycle;

  @override
  int get hashCode =>
      recoveryToken.hashCode + expiresAt.hashCode + lifecycle.hashCode;

  factory RecoveryCredential.fromJson(Map<String, dynamic> json) =>
      _$RecoveryCredentialFromJson(json);

  Map<String, dynamic> toJson() => _$RecoveryCredentialToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
