// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'credit_reconciliation_wallet.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CreditReconciliationWallet _$CreditReconciliationWalletFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('CreditReconciliationWallet', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'accountId',
      'expectedBalance',
      'delta',
      'expectedLastSeq',
      'walletExists',
      'hasBalanceDrift',
      'hasSequenceDrift',
    ],
  );
  final val = CreditReconciliationWallet(
    accountId: $checkedConvert('accountId', (v) => v as String),
    expectedBalance: $checkedConvert('expectedBalance', (v) => v as String),
    actualBalance: $checkedConvert('actualBalance', (v) => v as String?),
    delta: $checkedConvert('delta', (v) => v as String),
    expectedLastSeq: $checkedConvert(
      'expectedLastSeq',
      (v) => (v as num).toInt(),
    ),
    actualLastSeq: $checkedConvert(
      'actualLastSeq',
      (v) => (v as num?)?.toInt(),
    ),
    walletExists: $checkedConvert('walletExists', (v) => v as bool),
    hasBalanceDrift: $checkedConvert('hasBalanceDrift', (v) => v as bool),
    hasSequenceDrift: $checkedConvert('hasSequenceDrift', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$CreditReconciliationWalletToJson(
  CreditReconciliationWallet instance,
) => <String, dynamic>{
  'accountId': instance.accountId,
  'expectedBalance': instance.expectedBalance,
  'actualBalance': ?instance.actualBalance,
  'delta': instance.delta,
  'expectedLastSeq': instance.expectedLastSeq,
  'actualLastSeq': ?instance.actualLastSeq,
  'walletExists': instance.walletExists,
  'hasBalanceDrift': instance.hasBalanceDrift,
  'hasSequenceDrift': instance.hasSequenceDrift,
};
