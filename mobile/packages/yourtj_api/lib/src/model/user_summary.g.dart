// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_summary.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserSummary _$UserSummaryFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UserSummary', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'handle',
          'displayName',
          'avatarUrl',
          'role',
          'followedAt',
        ],
      );
      final val = UserSummary(
        id: $checkedConvert('id', (v) => v as String),
        handle: $checkedConvert('handle', (v) => v as String),
        displayName: $checkedConvert('displayName', (v) => v as String?),
        avatarUrl: $checkedConvert('avatarUrl', (v) => v as String?),
        role: $checkedConvert(
          'role',
          (v) => $enumDecode(
            _$UserSummaryRoleEnumEnumMap,
            v,
            unknownValue: UserSummaryRoleEnum.unknownDefaultOpenApi,
          ),
        ),
        followedAt: $checkedConvert('followedAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$UserSummaryToJson(UserSummary instance) =>
    <String, dynamic>{
      'id': instance.id,
      'handle': instance.handle,
      'displayName': instance.displayName,
      'avatarUrl': instance.avatarUrl,
      'role': _$UserSummaryRoleEnumEnumMap[instance.role]!,
      'followedAt': instance.followedAt,
    };

const _$UserSummaryRoleEnumEnumMap = {
  UserSummaryRoleEnum.user: 'user',
  UserSummaryRoleEnum.mod: 'mod',
  UserSummaryRoleEnum.admin: 'admin',
  UserSummaryRoleEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
