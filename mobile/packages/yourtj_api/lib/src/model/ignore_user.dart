//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'ignore_user.g.dart';

@Deprecated('IgnoreUser has been deprecated')
@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class IgnoreUser {
  /// Returns a new [IgnoreUser] instance.
  IgnoreUser({this.accountId, this.handle, this.avatarUrl, this.createdAt});

  @JsonKey(name: r'accountId', required: false, includeIfNull: false)
  final String? accountId;

  @JsonKey(name: r'handle', required: false, includeIfNull: false)
  final String? handle;

  /// Short-lived clean thumb_256 compatibility URL; refresh the owning ignore-list response after expiry.
  @JsonKey(name: r'avatarUrl', required: false, includeIfNull: false)
  final String? avatarUrl;

  @JsonKey(name: r'createdAt', required: false, includeIfNull: false)
  final int? createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is IgnoreUser &&
          other.accountId == accountId &&
          other.handle == handle &&
          other.avatarUrl == avatarUrl &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      accountId.hashCode +
      handle.hashCode +
      (avatarUrl == null ? 0 : avatarUrl.hashCode) +
      createdAt.hashCode;

  factory IgnoreUser.fromJson(Map<String, dynamic> json) =>
      _$IgnoreUserFromJson(json);

  Map<String, dynamic> toJson() => _$IgnoreUserToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
