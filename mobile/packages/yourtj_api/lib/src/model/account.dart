//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'account.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Account {
  /// Returns a new [Account] instance.
  Account({
    required this.id,

    required this.handle,

    required this.avatarUrl,

    required this.role,

    required this.capabilities,

    required this.trustLevel,

    required this.hasPassword,

    required this.onboardingRequired,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'handle', required: true, includeIfNull: false)
  final String handle;

  /// Legacy compatibility field; profile images are controlled media assets.
  @Deprecated('avatarUrl has been deprecated')
  @JsonKey(name: r'avatarUrl', required: true, includeIfNull: true)
  final String? avatarUrl;

  @JsonKey(
    name: r'role',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AccountRoleEnum.unknownDefaultOpenApi,
  )
  final AccountRoleEnum role;

  @JsonKey(name: r'capabilities', required: true, includeIfNull: false)
  final List<String> capabilities;

  /// Unified tea trust level. 0 is visitor UI only; registered accounts are 1–6.
  // minimum: 0
  // maximum: 6
  @JsonKey(name: r'trustLevel', required: true, includeIfNull: false)
  final int trustLevel;

  /// Owner-only credential state used to choose between first-time password setup and password change.
  @JsonKey(name: r'hasPassword', required: true, includeIfNull: false)
  final bool hasPassword;

  /// True until the current terms version and required profile/privacy choices are accepted.
  @JsonKey(name: r'onboardingRequired', required: true, includeIfNull: false)
  final bool onboardingRequired;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Account &&
          other.id == id &&
          other.handle == handle &&
          other.avatarUrl == avatarUrl &&
          other.role == role &&
          other.capabilities == capabilities &&
          other.trustLevel == trustLevel &&
          other.hasPassword == hasPassword &&
          other.onboardingRequired == onboardingRequired &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      handle.hashCode +
      (avatarUrl == null ? 0 : avatarUrl.hashCode) +
      role.hashCode +
      capabilities.hashCode +
      trustLevel.hashCode +
      hasPassword.hashCode +
      onboardingRequired.hashCode +
      createdAt.hashCode;

  factory Account.fromJson(Map<String, dynamic> json) =>
      _$AccountFromJson(json);

  Map<String, dynamic> toJson() => _$AccountToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AccountRoleEnum {
  @JsonValue(r'user')
  user(r'user'),
  @JsonValue(r'mod')
  mod(r'mod'),
  @JsonValue(r'admin')
  admin(r'admin'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AccountRoleEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
