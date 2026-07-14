// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'wallet.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Wallet _$WalletFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Wallet', json, ($checkedConvert) {
      final val = Wallet(
        accountId: $checkedConvert('accountId', (v) => v as String?),
        balance: $checkedConvert('balance', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$WalletToJson(Wallet instance) => <String, dynamic>{
  'accountId': ?instance.accountId,
  'balance': ?instance.balance,
};
