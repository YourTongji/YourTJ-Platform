// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'recent_auth_status.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

RecentAuthStatus _$RecentAuthStatusFromJson(Map<String, dynamic> json) =>
    $checkedCreate('RecentAuthStatus', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'sessionBound',
          'isFresh',
          'authenticatedAt',
          'expiresAt',
          'method',
          'availableMethods',
        ],
      );
      final val = RecentAuthStatus(
        sessionBound: $checkedConvert('sessionBound', (v) => v as bool),
        isFresh: $checkedConvert('isFresh', (v) => v as bool),
        authenticatedAt: $checkedConvert(
          'authenticatedAt',
          (v) => (v as num?)?.toInt(),
        ),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num?)?.toInt()),
        method: $checkedConvert(
          'method',
          (v) => $enumDecodeNullable(
            _$RecentAuthMethodEnumMap,
            v,
            unknownValue: RecentAuthMethod.unknownDefaultOpenApi,
          ),
        ),
        availableMethods: $checkedConvert(
          'availableMethods',
          (v) => (v as List<dynamic>)
              .map((e) => $enumDecode(_$RecentAuthMethodEnumMap, e))
              .toList(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$RecentAuthStatusToJson(RecentAuthStatus instance) =>
    <String, dynamic>{
      'sessionBound': instance.sessionBound,
      'isFresh': instance.isFresh,
      'authenticatedAt': instance.authenticatedAt,
      'expiresAt': instance.expiresAt,
      'method': _$RecentAuthMethodEnumMap[instance.method],
      'availableMethods': instance.availableMethods
          .map((e) => _$RecentAuthMethodEnumMap[e]!)
          .toList(),
    };

const _$RecentAuthMethodEnumMap = {
  RecentAuthMethod.password: 'password',
  RecentAuthMethod.emailCode: 'email_code',
  RecentAuthMethod.unknownDefaultOpenApi: 'unknown_default_open_api',
};
