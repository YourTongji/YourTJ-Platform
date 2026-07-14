//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'wallet_claim_challenge.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class WalletClaimChallenge {
  /// Returns a new [WalletClaimChallenge] instance.
  WalletClaimChallenge({required this.challengeId, required this.nonce});

  @JsonKey(name: r'challengeId', required: true, includeIfNull: false)
  final String challengeId;

  @JsonKey(name: r'nonce', required: true, includeIfNull: false)
  final String nonce;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is WalletClaimChallenge &&
          other.challengeId == challengeId &&
          other.nonce == nonce;

  @override
  int get hashCode => challengeId.hashCode + nonce.hashCode;

  factory WalletClaimChallenge.fromJson(Map<String, dynamic> json) =>
      _$WalletClaimChallengeFromJson(json);

  Map<String, dynamic> toJson() => _$WalletClaimChallengeToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
