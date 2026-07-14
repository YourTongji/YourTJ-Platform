// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_profile.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserProfile _$UserProfileFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('UserProfile', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'handle',
      'displayName',
      'school',
      'bio',
      'website',
      'avatarUrl',
      'bannerUrl',
      'role',
      'trustLevel',
      'badges',
      'verifications',
      'threadCount',
      'commentCount',
      'votesReceived',
      'followerCount',
      'followingCount',
      'canViewActivity',
      'createdAt',
    ],
  );
  final val = UserProfile(
    id: $checkedConvert('id', (v) => v as String),
    handle: $checkedConvert('handle', (v) => v as String),
    displayName: $checkedConvert('displayName', (v) => v as String?),
    school: $checkedConvert('school', (v) => v as String),
    bio: $checkedConvert('bio', (v) => v as String?),
    website: $checkedConvert('website', (v) => v as String?),
    avatarUrl: $checkedConvert('avatarUrl', (v) => v as String?),
    bannerUrl: $checkedConvert('bannerUrl', (v) => v as String?),
    role: $checkedConvert(
      'role',
      (v) => $enumDecode(
        _$UserProfileRoleEnumEnumMap,
        v,
        unknownValue: UserProfileRoleEnum.unknownDefaultOpenApi,
      ),
    ),
    trustLevel: $checkedConvert('trustLevel', (v) => (v as num).toInt()),
    badges: $checkedConvert(
      'badges',
      (v) => (v as List<dynamic>)
          .map((e) => UserBadge.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    verifications: $checkedConvert(
      'verifications',
      (v) => (v as List<dynamic>)
          .map((e) => PublicVerification.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    threadCount: $checkedConvert('threadCount', (v) => (v as num).toInt()),
    commentCount: $checkedConvert('commentCount', (v) => (v as num).toInt()),
    votesReceived: $checkedConvert('votesReceived', (v) => (v as num).toInt()),
    followerCount: $checkedConvert('followerCount', (v) => (v as num).toInt()),
    followingCount: $checkedConvert(
      'followingCount',
      (v) => (v as num).toInt(),
    ),
    canViewActivity: $checkedConvert('canViewActivity', (v) => v as bool),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$UserProfileToJson(UserProfile instance) =>
    <String, dynamic>{
      'id': instance.id,
      'handle': instance.handle,
      'displayName': instance.displayName,
      'school': instance.school,
      'bio': instance.bio,
      'website': instance.website,
      'avatarUrl': instance.avatarUrl,
      'bannerUrl': instance.bannerUrl,
      'role': _$UserProfileRoleEnumEnumMap[instance.role]!,
      'trustLevel': instance.trustLevel,
      'badges': instance.badges.map((e) => e.toJson()).toList(),
      'verifications': instance.verifications.map((e) => e.toJson()).toList(),
      'threadCount': instance.threadCount,
      'commentCount': instance.commentCount,
      'votesReceived': instance.votesReceived,
      'followerCount': instance.followerCount,
      'followingCount': instance.followingCount,
      'canViewActivity': instance.canViewActivity,
      'createdAt': instance.createdAt,
    };

const _$UserProfileRoleEnumEnumMap = {
  UserProfileRoleEnum.user: 'user',
  UserProfileRoleEnum.mod: 'mod',
  UserProfileRoleEnum.admin: 'admin',
  UserProfileRoleEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
