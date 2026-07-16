// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'wallet_bind_post_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

WalletBindPostRequest _$WalletBindPostRequestFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('WalletBindPostRequest', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['publicKey']);
  final val = WalletBindPostRequest(
    accountId: $checkedConvert('accountId', (v) => v as String?),
    publicKey: $checkedConvert('publicKey', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$WalletBindPostRequestToJson(
  WalletBindPostRequest instance,
) => <String, dynamic>{
  'accountId': ?instance.accountId,
  'publicKey': instance.publicKey,
};
