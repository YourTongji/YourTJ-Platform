// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'wallet_claim_post_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

WalletClaimPostRequest _$WalletClaimPostRequestFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('WalletClaimPostRequest', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const ['legacyUserHash', 'challengeId', 'signature'],
  );
  final val = WalletClaimPostRequest(
    legacyUserHash: $checkedConvert('legacyUserHash', (v) => v as String),
    challengeId: $checkedConvert('challengeId', (v) => v as String),
    signature: $checkedConvert('signature', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$WalletClaimPostRequestToJson(
  WalletClaimPostRequest instance,
) => <String, dynamic>{
  'legacyUserHash': instance.legacyUserHash,
  'challengeId': instance.challengeId,
  'signature': instance.signature,
};
