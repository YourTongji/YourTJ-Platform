//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'wallet_bind_post_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class WalletBindPostRequest {
  /// Returns a new [WalletBindPostRequest] instance.
  WalletBindPostRequest({this.accountId, required this.publicKey});

  /// Canonical account identifier captured with the authenticated session; required for initial enrollment and optional only for legacy idempotent confirmation of the exact active key
  @JsonKey(name: r'accountId', required: false, includeIfNull: false)
  final String? accountId;

  /// Standard base64 encoding of a 32-byte Ed25519 public key
  @JsonKey(name: r'publicKey', required: true, includeIfNull: false)
  final String publicKey;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is WalletBindPostRequest &&
          other.accountId == accountId &&
          other.publicKey == publicKey;

  @override
  int get hashCode => accountId.hashCode + publicKey.hashCode;

  factory WalletBindPostRequest.fromJson(Map<String, dynamic> json) =>
      _$WalletBindPostRequestFromJson(json);

  Map<String, dynamic> toJson() => _$WalletBindPostRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
