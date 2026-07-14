// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_search_hit.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserSearchHit _$UserSearchHitFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UserSearchHit', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'handle',
          'displayName',
          'avatarUrl',
          'role',
          'followerCount',
          'following',
        ],
      );
      final val = UserSearchHit(
        id: $checkedConvert('id', (v) => v as String),
        handle: $checkedConvert('handle', (v) => v as String),
        displayName: $checkedConvert('displayName', (v) => v as String?),
        avatarUrl: $checkedConvert('avatarUrl', (v) => v as String?),
        role: $checkedConvert(
          'role',
          (v) => $enumDecode(
            _$UserSearchHitRoleEnumEnumMap,
            v,
            unknownValue: UserSearchHitRoleEnum.unknownDefaultOpenApi,
          ),
        ),
        followerCount: $checkedConvert(
          'followerCount',
          (v) => (v as num).toInt(),
        ),
        following: $checkedConvert('following', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$UserSearchHitToJson(UserSearchHit instance) =>
    <String, dynamic>{
      'id': instance.id,
      'handle': instance.handle,
      'displayName': instance.displayName,
      'avatarUrl': instance.avatarUrl,
      'role': _$UserSearchHitRoleEnumEnumMap[instance.role]!,
      'followerCount': instance.followerCount,
      'following': instance.following,
    };

const _$UserSearchHitRoleEnumEnumMap = {
  UserSearchHitRoleEnum.user: 'user',
  UserSearchHitRoleEnum.mod: 'mod',
  UserSearchHitRoleEnum.admin: 'admin',
  UserSearchHitRoleEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
