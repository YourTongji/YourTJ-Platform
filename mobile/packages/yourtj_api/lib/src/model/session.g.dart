// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'session.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Session _$SessionFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Session', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'isCurrent',
          'createdAt',
          'lastUsedAt',
          'expiresAt',
        ],
      );
      final val = Session(
        id: $checkedConvert('id', (v) => v as String),
        isCurrent: $checkedConvert('isCurrent', (v) => v as bool),
        deviceLabel: $checkedConvert('deviceLabel', (v) => v as String?),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
        lastUsedAt: $checkedConvert('lastUsedAt', (v) => (v as num).toInt()),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$SessionToJson(Session instance) => <String, dynamic>{
  'id': instance.id,
  'isCurrent': instance.isCurrent,
  'deviceLabel': ?instance.deviceLabel,
  'createdAt': instance.createdAt,
  'lastUsedAt': instance.lastUsedAt,
  'expiresAt': instance.expiresAt,
};
