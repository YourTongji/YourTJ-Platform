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
    publicKey: $checkedConvert('publicKey', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$WalletBindPostRequestToJson(
  WalletBindPostRequest instance,
) => <String, dynamic>{'publicKey': instance.publicKey};
