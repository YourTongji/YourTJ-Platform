//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'wallet.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class Wallet {
  /// Returns a new [Wallet] instance.
  Wallet({
    required this.accountId,

    required this.balance,

    required this.activePublicKey,
  });

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(name: r'balance', required: true, includeIfNull: false)
  final int balance;

  /// Standard base64 encoding of the account's active 32-byte Ed25519 public key; null until the first key is enrolled.
  @JsonKey(name: r'activePublicKey', required: true, includeIfNull: true)
  final String? activePublicKey;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Wallet &&
          other.accountId == accountId &&
          other.balance == balance &&
          other.activePublicKey == activePublicKey;

  @override
  int get hashCode =>
      accountId.hashCode +
      balance.hashCode +
      (activePublicKey == null ? 0 : activePublicKey.hashCode);

  factory Wallet.fromJson(Map<String, dynamic> json) => _$WalletFromJson(json);

  Map<String, dynamic> toJson() => _$WalletToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
