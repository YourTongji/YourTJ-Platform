// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'account.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Account _$AccountFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Account', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'handle',
          'avatarUrl',
          'role',
          'capabilities',
          'trustLevel',
          'hasPassword',
          'onboardingRequired',
          'createdAt',
        ],
      );
      final val = Account(
        id: $checkedConvert('id', (v) => v as String),
        handle: $checkedConvert('handle', (v) => v as String),
        avatarUrl: $checkedConvert('avatarUrl', (v) => v as String?),
        role: $checkedConvert(
          'role',
          (v) => $enumDecode(
            _$AccountRoleEnumEnumMap,
            v,
            unknownValue: AccountRoleEnum.unknownDefaultOpenApi,
          ),
        ),
        capabilities: $checkedConvert(
          'capabilities',
          (v) => (v as List<dynamic>).map((e) => e as String).toList(),
        ),
        trustLevel: $checkedConvert('trustLevel', (v) => (v as num).toInt()),
        hasPassword: $checkedConvert('hasPassword', (v) => v as bool),
        onboardingRequired: $checkedConvert(
          'onboardingRequired',
          (v) => v as bool,
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$AccountToJson(Account instance) => <String, dynamic>{
  'id': instance.id,
  'handle': instance.handle,
  'avatarUrl': instance.avatarUrl,
  'role': _$AccountRoleEnumEnumMap[instance.role]!,
  'capabilities': instance.capabilities,
  'trustLevel': instance.trustLevel,
  'hasPassword': instance.hasPassword,
  'onboardingRequired': instance.onboardingRequired,
  'createdAt': instance.createdAt,
};

const _$AccountRoleEnumEnumMap = {
  AccountRoleEnum.user: 'user',
  AccountRoleEnum.mod: 'mod',
  AccountRoleEnum.admin: 'admin',
  AccountRoleEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
