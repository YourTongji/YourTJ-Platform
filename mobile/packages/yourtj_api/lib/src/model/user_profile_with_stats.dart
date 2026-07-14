//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/user_badge.dart';
import 'package:yourtj_api/src/model/public_verification.dart';
import 'package:json_annotation/json_annotation.dart';

part 'user_profile_with_stats.g.dart';

@Deprecated('UserProfileWithStats has been deprecated')
@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UserProfileWithStats {
  /// Returns a new [UserProfileWithStats] instance.
  UserProfileWithStats({
    required this.id,

    required this.handle,

    required this.displayName,

    required this.school,

    required this.bio,

    required this.website,

    required this.avatarUrl,

    required this.bannerUrl,

    required this.role,

    required this.trustLevel,

    required this.badges,

    required this.verifications,

    required this.threadCount,

    required this.commentCount,

    required this.votesReceived,

    required this.followerCount,

    required this.followingCount,

    required this.canViewActivity,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'handle', required: true, includeIfNull: false)
  final String handle;

  @JsonKey(name: r'displayName', required: true, includeIfNull: true)
  final String? displayName;

  /// Owner-editable public school label governed by profile visibility.
  @JsonKey(name: r'school', required: true, includeIfNull: false)
  final String school;

  @JsonKey(name: r'bio', required: true, includeIfNull: true)
  final String? bio;

  @JsonKey(name: r'website', required: true, includeIfNull: true)
  final String? website;

  /// Short-lived clean thumb_256 compatibility URL; refresh the owning profile response after expiry.
  @JsonKey(name: r'avatarUrl', required: true, includeIfNull: true)
  final String? avatarUrl;

  @JsonKey(name: r'bannerUrl', required: true, includeIfNull: true)
  final String? bannerUrl;

  @JsonKey(
    name: r'role',
    required: true,
    includeIfNull: false,
    unknownEnumValue: UserProfileWithStatsRoleEnum.unknownDefaultOpenApi,
  )
  final UserProfileWithStatsRoleEnum role;

  // minimum: 0
  // maximum: 6
  @JsonKey(name: r'trustLevel', required: true, includeIfNull: false)
  final int trustLevel;

  @JsonKey(name: r'badges', required: true, includeIfNull: false)
  final List<UserBadge> badges;

  /// Active grants explicitly allowed for public profile display; never contains evidence, issuer, or staff reason.
  @JsonKey(name: r'verifications', required: true, includeIfNull: false)
  final List<PublicVerification> verifications;

  // minimum: 0
  @JsonKey(name: r'threadCount', required: true, includeIfNull: false)
  final int threadCount;

  // minimum: 0
  @JsonKey(name: r'commentCount', required: true, includeIfNull: false)
  final int commentCount;

  // minimum: 0
  @JsonKey(name: r'votesReceived', required: true, includeIfNull: false)
  final int votesReceived;

  // minimum: 0
  @JsonKey(name: r'followerCount', required: true, includeIfNull: false)
  final int followerCount;

  // minimum: 0
  @JsonKey(name: r'followingCount', required: true, includeIfNull: false)
  final int followingCount;

  /// Viewer-specific permission for authored-content lists and future activity, media, and likes tabs. Aggregate public-content counters remain visible with the profile.
  @JsonKey(name: r'canViewActivity', required: true, includeIfNull: false)
  final bool canViewActivity;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UserProfileWithStats &&
          other.id == id &&
          other.handle == handle &&
          other.displayName == displayName &&
          other.school == school &&
          other.bio == bio &&
          other.website == website &&
          other.avatarUrl == avatarUrl &&
          other.bannerUrl == bannerUrl &&
          other.role == role &&
          other.trustLevel == trustLevel &&
          other.badges == badges &&
          other.verifications == verifications &&
          other.threadCount == threadCount &&
          other.commentCount == commentCount &&
          other.votesReceived == votesReceived &&
          other.followerCount == followerCount &&
          other.followingCount == followingCount &&
          other.canViewActivity == canViewActivity &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      handle.hashCode +
      (displayName == null ? 0 : displayName.hashCode) +
      school.hashCode +
      (bio == null ? 0 : bio.hashCode) +
      (website == null ? 0 : website.hashCode) +
      (avatarUrl == null ? 0 : avatarUrl.hashCode) +
      (bannerUrl == null ? 0 : bannerUrl.hashCode) +
      role.hashCode +
      trustLevel.hashCode +
      badges.hashCode +
      verifications.hashCode +
      threadCount.hashCode +
      commentCount.hashCode +
      votesReceived.hashCode +
      followerCount.hashCode +
      followingCount.hashCode +
      canViewActivity.hashCode +
      createdAt.hashCode;

  factory UserProfileWithStats.fromJson(Map<String, dynamic> json) =>
      _$UserProfileWithStatsFromJson(json);

  Map<String, dynamic> toJson() => _$UserProfileWithStatsToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

@Deprecated('UserProfileWithStatsRoleEnum has been deprecated')
enum UserProfileWithStatsRoleEnum {
  @JsonValue(r'user')
  user(r'user'),
  @JsonValue(r'mod')
  mod(r'mod'),
  @JsonValue(r'admin')
  admin(r'admin'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UserProfileWithStatsRoleEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
