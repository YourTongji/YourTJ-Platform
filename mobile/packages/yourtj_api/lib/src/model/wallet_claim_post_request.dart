//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'wallet_claim_post_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class WalletClaimPostRequest {
  /// Returns a new [WalletClaimPostRequest] instance.
  WalletClaimPostRequest({
    required this.legacyUserHash,

    required this.challengeId,

    required this.signature,
  });

  /// Canonical lowercase hexadecimal SHA-256 legacy user hash
  @JsonKey(name: r'legacyUserHash', required: true, includeIfNull: false)
  final String legacyUserHash;

  /// Canonical lowercase UUID v4 returned by the latest claim-challenge request
  @JsonKey(name: r'challengeId', required: true, includeIfNull: false)
  final String challengeId;

  /// Canonical standard-base64 encoding of a 64-byte Ed25519 signature
  @JsonKey(name: r'signature', required: true, includeIfNull: false)
  final String signature;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is WalletClaimPostRequest &&
          other.legacyUserHash == legacyUserHash &&
          other.challengeId == challengeId &&
          other.signature == signature;

  @override
  int get hashCode =>
      legacyUserHash.hashCode + challengeId.hashCode + signature.hashCode;

  factory WalletClaimPostRequest.fromJson(Map<String, dynamic> json) =>
      _$WalletClaimPostRequestFromJson(json);

  Map<String, dynamic> toJson() => _$WalletClaimPostRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
