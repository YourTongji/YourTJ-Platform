// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_user.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminUser _$AdminUserFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminUser', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'handle',
          'role',
          'status',
          'trustLevel',
          'createdAt',
        ],
      );
      final val = AdminUser(
        id: $checkedConvert('id', (v) => v as String),
        handle: $checkedConvert('handle', (v) => v as String),
        avatarUrl: $checkedConvert('avatarUrl', (v) => v as String?),
        role: $checkedConvert(
          'role',
          (v) => $enumDecode(
            _$AdminUserRoleEnumEnumMap,
            v,
            unknownValue: AdminUserRoleEnum.unknownDefaultOpenApi,
          ),
        ),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$AdminUserStatusEnumEnumMap,
            v,
            unknownValue: AdminUserStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        trustLevel: $checkedConvert('trustLevel', (v) => (v as num).toInt()),
        lastActiveAt: $checkedConvert(
          'lastActiveAt',
          (v) => (v as num?)?.toInt(),
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$AdminUserToJson(AdminUser instance) => <String, dynamic>{
  'id': instance.id,
  'handle': instance.handle,
  'avatarUrl': ?instance.avatarUrl,
  'role': _$AdminUserRoleEnumEnumMap[instance.role]!,
  'status': _$AdminUserStatusEnumEnumMap[instance.status]!,
  'trustLevel': instance.trustLevel,
  'lastActiveAt': ?instance.lastActiveAt,
  'createdAt': instance.createdAt,
};

const _$AdminUserRoleEnumEnumMap = {
  AdminUserRoleEnum.user: 'user',
  AdminUserRoleEnum.mod: 'mod',
  AdminUserRoleEnum.admin: 'admin',
  AdminUserRoleEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AdminUserStatusEnumEnumMap = {
  AdminUserStatusEnum.active: 'active',
  AdminUserStatusEnum.suspended: 'suspended',
  AdminUserStatusEnum.deactivated: 'deactivated',
  AdminUserStatusEnum.deletionRequested: 'deletion_requested',
  AdminUserStatusEnum.deleted: 'deleted',
  AdminUserStatusEnum.purged: 'purged',
  AdminUserStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
