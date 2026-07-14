// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ledger_entry.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

LedgerEntry _$LedgerEntryFromJson(Map<String, dynamic> json) =>
    $checkedCreate('LedgerEntry', json, ($checkedConvert) {
      final val = LedgerEntry(
        seq: $checkedConvert('seq', (v) => (v as num?)?.toInt()),
        txId: $checkedConvert('txId', (v) => v as String?),
        type: $checkedConvert(
          'type',
          (v) => $enumDecodeNullable(
            _$LedgerEntryTypeEnumEnumMap,
            v,
            unknownValue: LedgerEntryTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        fromAccount: $checkedConvert('fromAccount', (v) => v as String?),
        toAccount: $checkedConvert('toAccount', (v) => v as String?),
        amount: $checkedConvert('amount', (v) => (v as num?)?.toInt()),
        hash: $checkedConvert('hash', (v) => v as String?),
        createdAt: $checkedConvert('createdAt', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$LedgerEntryToJson(LedgerEntry instance) =>
    <String, dynamic>{
      'seq': ?instance.seq,
      'txId': ?instance.txId,
      'type': ?_$LedgerEntryTypeEnumEnumMap[instance.type],
      'fromAccount': ?instance.fromAccount,
      'toAccount': ?instance.toAccount,
      'amount': ?instance.amount,
      'hash': ?instance.hash,
      'createdAt': ?instance.createdAt,
    };

const _$LedgerEntryTypeEnumEnumMap = {
  LedgerEntryTypeEnum.mint: 'mint',
  LedgerEntryTypeEnum.tip: 'tip',
  LedgerEntryTypeEnum.escrowHold: 'escrow_hold',
  LedgerEntryTypeEnum.escrowRelease: 'escrow_release',
  LedgerEntryTypeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
