// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ledger_verify.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

LedgerVerify _$LedgerVerifyFromJson(Map<String, dynamic> json) =>
    $checkedCreate('LedgerVerify', json, ($checkedConvert) {
      final val = LedgerVerify(
        ok: $checkedConvert('ok', (v) => v as bool?),
        latestSeq: $checkedConvert('latestSeq', (v) => (v as num?)?.toInt()),
        latestHash: $checkedConvert('latestHash', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$LedgerVerifyToJson(LedgerVerify instance) =>
    <String, dynamic>{
      'ok': ?instance.ok,
      'latestSeq': ?instance.latestSeq,
      'latestHash': ?instance.latestHash,
    };
