// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'wallet_claim_challenge.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

WalletClaimChallenge _$WalletClaimChallengeFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('WalletClaimChallenge', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['challengeId', 'nonce']);
  final val = WalletClaimChallenge(
    challengeId: $checkedConvert('challengeId', (v) => v as String),
    nonce: $checkedConvert('nonce', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$WalletClaimChallengeToJson(
  WalletClaimChallenge instance,
) => <String, dynamic>{
  'challengeId': instance.challengeId,
  'nonce': instance.nonce,
};
