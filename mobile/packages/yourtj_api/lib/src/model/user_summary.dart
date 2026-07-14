//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'user_summary.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UserSummary {
  /// Returns a new [UserSummary] instance.
  UserSummary({
    required this.id,

    required this.handle,

    required this.displayName,

    required this.avatarUrl,

    required this.role,

    required this.followedAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'handle', required: true, includeIfNull: false)
  final String handle;

  @JsonKey(name: r'displayName', required: true, includeIfNull: true)
  final String? displayName;

  /// Short-lived clean thumb_256 compatibility URL; refresh the owning relationship-list response after expiry.
  @JsonKey(name: r'avatarUrl', required: true, includeIfNull: true)
  final String? avatarUrl;

  @JsonKey(
    name: r'role',
    required: true,
    includeIfNull: false,
    unknownEnumValue: UserSummaryRoleEnum.unknownDefaultOpenApi,
  )
  final UserSummaryRoleEnum role;

  @JsonKey(name: r'followedAt', required: true, includeIfNull: false)
  final int followedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UserSummary &&
          other.id == id &&
          other.handle == handle &&
          other.displayName == displayName &&
          other.avatarUrl == avatarUrl &&
          other.role == role &&
          other.followedAt == followedAt;

  @override
  int get hashCode =>
      id.hashCode +
      handle.hashCode +
      (displayName == null ? 0 : displayName.hashCode) +
      (avatarUrl == null ? 0 : avatarUrl.hashCode) +
      role.hashCode +
      followedAt.hashCode;

  factory UserSummary.fromJson(Map<String, dynamic> json) =>
      _$UserSummaryFromJson(json);

  Map<String, dynamic> toJson() => _$UserSummaryToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum UserSummaryRoleEnum {
  @JsonValue(r'user')
  user(r'user'),
  @JsonValue(r'mod')
  mod(r'mod'),
  @JsonValue(r'admin')
  admin(r'admin'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UserSummaryRoleEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
