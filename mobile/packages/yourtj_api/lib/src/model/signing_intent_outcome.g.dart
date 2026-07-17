// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'signing_intent_outcome.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SigningIntentOutcome _$SigningIntentOutcomeFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('SigningIntentOutcome', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['intentId', 'status', 'expiresAt']);
  final val = SigningIntentOutcome(
    intentId: $checkedConvert('intentId', (v) => v as String),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$SigningIntentOutcomeStatusEnumEnumMap,
        v,
        unknownValue: SigningIntentOutcomeStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$SigningIntentOutcomeToJson(
  SigningIntentOutcome instance,
) => <String, dynamic>{
  'intentId': instance.intentId,
  'status': _$SigningIntentOutcomeStatusEnumEnumMap[instance.status]!,
  'expiresAt': instance.expiresAt,
};

const _$SigningIntentOutcomeStatusEnumEnumMap = {
  SigningIntentOutcomeStatusEnum.pending: 'pending',
  SigningIntentOutcomeStatusEnum.committed: 'committed',
  SigningIntentOutcomeStatusEnum.expired: 'expired',
  SigningIntentOutcomeStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
