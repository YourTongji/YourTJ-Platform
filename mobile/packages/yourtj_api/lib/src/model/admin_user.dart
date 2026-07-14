//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_user.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminUser {
  /// Returns a new [AdminUser] instance.
  AdminUser({
    required this.id,

    required this.handle,

    this.avatarUrl,

    required this.role,

    required this.status,

    required this.trustLevel,

    this.lastActiveAt,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'handle', required: true, includeIfNull: false)
  final String handle;

  @JsonKey(name: r'avatarUrl', required: false, includeIfNull: false)
  final String? avatarUrl;

  @JsonKey(
    name: r'role',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminUserRoleEnum.unknownDefaultOpenApi,
  )
  final AdminUserRoleEnum role;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminUserStatusEnum.unknownDefaultOpenApi,
  )
  final AdminUserStatusEnum status;

  // minimum: 0
  // maximum: 6
  @JsonKey(name: r'trustLevel', required: true, includeIfNull: false)
  final int trustLevel;

  @JsonKey(name: r'lastActiveAt', required: false, includeIfNull: false)
  final int? lastActiveAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminUser &&
          other.id == id &&
          other.handle == handle &&
          other.avatarUrl == avatarUrl &&
          other.role == role &&
          other.status == status &&
          other.trustLevel == trustLevel &&
          other.lastActiveAt == lastActiveAt &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      handle.hashCode +
      (avatarUrl == null ? 0 : avatarUrl.hashCode) +
      role.hashCode +
      status.hashCode +
      trustLevel.hashCode +
      (lastActiveAt == null ? 0 : lastActiveAt.hashCode) +
      createdAt.hashCode;

  factory AdminUser.fromJson(Map<String, dynamic> json) =>
      _$AdminUserFromJson(json);

  Map<String, dynamic> toJson() => _$AdminUserToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AdminUserRoleEnum {
  @JsonValue(r'user')
  user(r'user'),
  @JsonValue(r'mod')
  mod(r'mod'),
  @JsonValue(r'admin')
  admin(r'admin'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminUserRoleEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AdminUserStatusEnum {
  @JsonValue(r'active')
  active(r'active'),
  @JsonValue(r'suspended')
  suspended(r'suspended'),
  @JsonValue(r'deactivated')
  deactivated(r'deactivated'),
  @JsonValue(r'deletion_requested')
  deletionRequested(r'deletion_requested'),
  @JsonValue(r'deleted')
  deleted(r'deleted'),
  @JsonValue(r'purged')
  purged(r'purged'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminUserStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
