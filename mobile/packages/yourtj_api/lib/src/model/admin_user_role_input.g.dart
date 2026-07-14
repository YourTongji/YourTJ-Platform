// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_user_role_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminUserRoleInput _$AdminUserRoleInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminUserRoleInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['role', 'reason']);
      final val = AdminUserRoleInput(
        role: $checkedConvert(
          'role',
          (v) => $enumDecode(
            _$AdminUserRoleInputRoleEnumEnumMap,
            v,
            unknownValue: AdminUserRoleInputRoleEnum.unknownDefaultOpenApi,
          ),
        ),
        reason: $checkedConvert('reason', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$AdminUserRoleInputToJson(AdminUserRoleInput instance) =>
    <String, dynamic>{
      'role': _$AdminUserRoleInputRoleEnumEnumMap[instance.role]!,
      'reason': instance.reason,
    };

const _$AdminUserRoleInputRoleEnumEnumMap = {
  AdminUserRoleInputRoleEnum.user: 'user',
  AdminUserRoleInputRoleEnum.mod: 'mod',
  AdminUserRoleInputRoleEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
