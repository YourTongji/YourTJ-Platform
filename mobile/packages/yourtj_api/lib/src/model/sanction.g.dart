// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'sanction.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Sanction _$SanctionFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Sanction', json, ($checkedConvert) {
      final val = Sanction(
        id: $checkedConvert('id', (v) => v as String?),
        accountId: $checkedConvert('accountId', (v) => v as String?),
        kind: $checkedConvert(
          'kind',
          (v) => $enumDecodeNullable(
            _$SanctionKindEnumEnumMap,
            v,
            unknownValue: SanctionKindEnum.unknownDefaultOpenApi,
          ),
        ),
        reason: $checkedConvert('reason', (v) => v as String?),
        issuedBy: $checkedConvert('issuedBy', (v) => v as String?),
        startsAt: $checkedConvert('startsAt', (v) => (v as num?)?.toInt()),
        endsAt: $checkedConvert('endsAt', (v) => (v as num?)?.toInt()),
        revokedAt: $checkedConvert('revokedAt', (v) => (v as num?)?.toInt()),
        createdAt: $checkedConvert('createdAt', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$SanctionToJson(Sanction instance) => <String, dynamic>{
  'id': ?instance.id,
  'accountId': ?instance.accountId,
  'kind': ?_$SanctionKindEnumEnumMap[instance.kind],
  'reason': ?instance.reason,
  'issuedBy': ?instance.issuedBy,
  'startsAt': ?instance.startsAt,
  'endsAt': ?instance.endsAt,
  'revokedAt': ?instance.revokedAt,
  'createdAt': ?instance.createdAt,
};

const _$SanctionKindEnumEnumMap = {
  SanctionKindEnum.silence: 'silence',
  SanctionKindEnum.suspend: 'suspend',
  SanctionKindEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
