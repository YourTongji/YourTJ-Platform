// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'credit_reconciliation_run.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CreditReconciliationRun _$CreditReconciliationRunFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('CreditReconciliationRun', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'status',
      'requestedBy',
      'reason',
      'walletsChecked',
      'driftedWallets',
      'missingWallets',
      'balanceDriftedWallets',
      'sequenceDriftedWallets',
      'totalAbsoluteDrift',
      'createdAt',
    ],
  );
  final val = CreditReconciliationRun(
    id: $checkedConvert('id', (v) => v as String),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$CreditReconciliationRunStatusEnumEnumMap,
        v,
        unknownValue: CreditReconciliationRunStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    requestedBy: $checkedConvert('requestedBy', (v) => v as String),
    reason: $checkedConvert('reason', (v) => v as String),
    ledgerOk: $checkedConvert('ledgerOk', (v) => v as bool?),
    ledgerLatestSeq: $checkedConvert(
      'ledgerLatestSeq',
      (v) => (v as num?)?.toInt(),
    ),
    ledgerLatestHash: $checkedConvert('ledgerLatestHash', (v) => v as String?),
    ledgerFailureSeq: $checkedConvert(
      'ledgerFailureSeq',
      (v) => (v as num?)?.toInt(),
    ),
    walletsChecked: $checkedConvert(
      'walletsChecked',
      (v) => (v as num).toInt(),
    ),
    driftedWallets: $checkedConvert(
      'driftedWallets',
      (v) => (v as num).toInt(),
    ),
    missingWallets: $checkedConvert(
      'missingWallets',
      (v) => (v as num).toInt(),
    ),
    balanceDriftedWallets: $checkedConvert(
      'balanceDriftedWallets',
      (v) => (v as num).toInt(),
    ),
    sequenceDriftedWallets: $checkedConvert(
      'sequenceDriftedWallets',
      (v) => (v as num).toInt(),
    ),
    totalAbsoluteDrift: $checkedConvert(
      'totalAbsoluteDrift',
      (v) => v as String,
    ),
    errorCode: $checkedConvert('errorCode', (v) => v as String?),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
    startedAt: $checkedConvert('startedAt', (v) => (v as num?)?.toInt()),
    completedAt: $checkedConvert('completedAt', (v) => (v as num?)?.toInt()),
  );
  return val;
});

Map<String, dynamic> _$CreditReconciliationRunToJson(
  CreditReconciliationRun instance,
) => <String, dynamic>{
  'id': instance.id,
  'status': _$CreditReconciliationRunStatusEnumEnumMap[instance.status]!,
  'requestedBy': instance.requestedBy,
  'reason': instance.reason,
  'ledgerOk': ?instance.ledgerOk,
  'ledgerLatestSeq': ?instance.ledgerLatestSeq,
  'ledgerLatestHash': ?instance.ledgerLatestHash,
  'ledgerFailureSeq': ?instance.ledgerFailureSeq,
  'walletsChecked': instance.walletsChecked,
  'driftedWallets': instance.driftedWallets,
  'missingWallets': instance.missingWallets,
  'balanceDriftedWallets': instance.balanceDriftedWallets,
  'sequenceDriftedWallets': instance.sequenceDriftedWallets,
  'totalAbsoluteDrift': instance.totalAbsoluteDrift,
  'errorCode': ?instance.errorCode,
  'createdAt': instance.createdAt,
  'startedAt': ?instance.startedAt,
  'completedAt': ?instance.completedAt,
};

const _$CreditReconciliationRunStatusEnumEnumMap = {
  CreditReconciliationRunStatusEnum.queued: 'queued',
  CreditReconciliationRunStatusEnum.running: 'running',
  CreditReconciliationRunStatusEnum.succeeded: 'succeeded',
  CreditReconciliationRunStatusEnum.failed: 'failed',
  CreditReconciliationRunStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
