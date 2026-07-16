// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'latest_update.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

LatestUpdate _$LatestUpdateFromJson(Map<String, dynamic> json) =>
    $checkedCreate('LatestUpdate', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'updatedAt',
          'importedAt',
          'stale',
          'staleAfterHours',
        ],
      );
      final val = LatestUpdate(
        updatedAt: $checkedConvert(
          'updatedAt',
          (v) => v == null ? null : DateTime.parse(v as String),
        ),
        importedAt: $checkedConvert(
          'importedAt',
          (v) => v == null ? null : DateTime.parse(v as String),
        ),
        stale: $checkedConvert('stale', (v) => v as bool),
        staleAfterHours: $checkedConvert(
          'staleAfterHours',
          (v) => $enumDecode(
            _$LatestUpdateStaleAfterHoursEnumEnumMap,
            v,
            unknownValue: LatestUpdateStaleAfterHoursEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$LatestUpdateToJson(LatestUpdate instance) =>
    <String, dynamic>{
      'updatedAt': instance.updatedAt?.toIso8601String(),
      'importedAt': instance.importedAt?.toIso8601String(),
      'stale': instance.stale,
      'staleAfterHours':
          _$LatestUpdateStaleAfterHoursEnumEnumMap[instance.staleAfterHours]!,
    };

const _$LatestUpdateStaleAfterHoursEnumEnumMap = {
  LatestUpdateStaleAfterHoursEnum.number168: 168,
  LatestUpdateStaleAfterHoursEnum.unknownDefaultOpenApi: 11184809,
};
