//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'my_profile.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MyProfile {
  /// Returns a new [MyProfile] instance.
  MyProfile({
    required this.accountId,

    required this.displayName,

    required this.school,

    required this.bio,

    required this.website,

    required this.avatarAssetId,

    required this.bannerAssetId,
  });

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(name: r'displayName', required: true, includeIfNull: true)
  final String? displayName;

  @JsonKey(name: r'school', required: true, includeIfNull: false)
  final String school;

  @JsonKey(name: r'bio', required: true, includeIfNull: true)
  final String? bio;

  @JsonKey(name: r'website', required: true, includeIfNull: true)
  final String? website;

  @JsonKey(name: r'avatarAssetId', required: true, includeIfNull: true)
  final String? avatarAssetId;

  @JsonKey(name: r'bannerAssetId', required: true, includeIfNull: true)
  final String? bannerAssetId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MyProfile &&
          other.accountId == accountId &&
          other.displayName == displayName &&
          other.school == school &&
          other.bio == bio &&
          other.website == website &&
          other.avatarAssetId == avatarAssetId &&
          other.bannerAssetId == bannerAssetId;

  @override
  int get hashCode =>
      accountId.hashCode +
      (displayName == null ? 0 : displayName.hashCode) +
      school.hashCode +
      (bio == null ? 0 : bio.hashCode) +
      (website == null ? 0 : website.hashCode) +
      (avatarAssetId == null ? 0 : avatarAssetId.hashCode) +
      (bannerAssetId == null ? 0 : bannerAssetId.hashCode);

  factory MyProfile.fromJson(Map<String, dynamic> json) =>
      _$MyProfileFromJson(json);

  Map<String, dynamic> toJson() => _$MyProfileToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
