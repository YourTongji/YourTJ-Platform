// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'recent_auth_verify_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

RecentAuthVerifyInput _$RecentAuthVerifyInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('RecentAuthVerifyInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['method']);
  final val = RecentAuthVerifyInput(
    method: $checkedConvert(
      'method',
      (v) => $enumDecode(
        _$RecentAuthMethodEnumMap,
        v,
        unknownValue: RecentAuthMethod.unknownDefaultOpenApi,
      ),
    ),
    password: $checkedConvert('password', (v) => v as String?),
    code: $checkedConvert('code', (v) => v as String?),
  );
  return val;
});

Map<String, dynamic> _$RecentAuthVerifyInputToJson(
  RecentAuthVerifyInput instance,
) => <String, dynamic>{
  'method': _$RecentAuthMethodEnumMap[instance.method]!,
  'password': ?instance.password,
  'code': ?instance.code,
};

const _$RecentAuthMethodEnumMap = {
  RecentAuthMethod.password: 'password',
  RecentAuthMethod.emailCode: 'email_code',
  RecentAuthMethod.unknownDefaultOpenApi: 'unknown_default_open_api',
};
