//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'user_search_hit.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UserSearchHit {
  /// Returns a new [UserSearchHit] instance.
  UserSearchHit({
    required this.id,

    required this.handle,

    required this.displayName,

    required this.avatarUrl,

    required this.role,

    required this.followerCount,

    required this.following,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'handle', required: true, includeIfNull: false)
  final String handle;

  @JsonKey(name: r'displayName', required: true, includeIfNull: true)
  final String? displayName;

  /// Short-lived clean thumb_256 compatibility URL; refresh the owning search response after expiry.
  @JsonKey(name: r'avatarUrl', required: true, includeIfNull: true)
  final String? avatarUrl;

  @JsonKey(
    name: r'role',
    required: true,
    includeIfNull: false,
    unknownEnumValue: UserSearchHitRoleEnum.unknownDefaultOpenApi,
  )
  final UserSearchHitRoleEnum role;

  // minimum: 0
  @JsonKey(name: r'followerCount', required: true, includeIfNull: false)
  final int followerCount;

  @JsonKey(name: r'following', required: true, includeIfNull: false)
  final bool following;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UserSearchHit &&
          other.id == id &&
          other.handle == handle &&
          other.displayName == displayName &&
          other.avatarUrl == avatarUrl &&
          other.role == role &&
          other.followerCount == followerCount &&
          other.following == following;

  @override
  int get hashCode =>
      id.hashCode +
      handle.hashCode +
      (displayName == null ? 0 : displayName.hashCode) +
      (avatarUrl == null ? 0 : avatarUrl.hashCode) +
      role.hashCode +
      followerCount.hashCode +
      following.hashCode;

  factory UserSearchHit.fromJson(Map<String, dynamic> json) =>
      _$UserSearchHitFromJson(json);

  Map<String, dynamic> toJson() => _$UserSearchHitToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum UserSearchHitRoleEnum {
  @JsonValue(r'user')
  user(r'user'),
  @JsonValue(r'mod')
  mod(r'mod'),
  @JsonValue(r'admin')
  admin(r'admin'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UserSearchHitRoleEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
