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
  Wallet({this.accountId, this.balance});

  @JsonKey(name: r'accountId', required: false, includeIfNull: false)
  final String? accountId;

  @JsonKey(name: r'balance', required: false, includeIfNull: false)
  final int? balance;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Wallet &&
          other.accountId == accountId &&
          other.balance == balance;

  @override
  int get hashCode => accountId.hashCode + balance.hashCode;

  factory Wallet.fromJson(Map<String, dynamic> json) => _$WalletFromJson(json);

  Map<String, dynamic> toJson() => _$WalletToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
