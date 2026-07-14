//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'credit_reconciliation_wallet.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CreditReconciliationWallet {
  /// Returns a new [CreditReconciliationWallet] instance.
  CreditReconciliationWallet({
    required this.accountId,

    required this.expectedBalance,

    this.actualBalance,

    required this.delta,

    required this.expectedLastSeq,

    this.actualLastSeq,

    required this.walletExists,

    required this.hasBalanceDrift,

    required this.hasSequenceDrift,
  });

  @JsonKey(name: r'accountId', required: true, includeIfNull: false)
  final String accountId;

  @JsonKey(name: r'expectedBalance', required: true, includeIfNull: false)
  final String expectedBalance;

  @JsonKey(name: r'actualBalance', required: false, includeIfNull: false)
  final String? actualBalance;

  @JsonKey(name: r'delta', required: true, includeIfNull: false)
  final String delta;

  // minimum: 0
  @JsonKey(name: r'expectedLastSeq', required: true, includeIfNull: false)
  final int expectedLastSeq;

  // minimum: 0
  @JsonKey(name: r'actualLastSeq', required: false, includeIfNull: false)
  final int? actualLastSeq;

  @JsonKey(name: r'walletExists', required: true, includeIfNull: false)
  final bool walletExists;

  @JsonKey(name: r'hasBalanceDrift', required: true, includeIfNull: false)
  final bool hasBalanceDrift;

  @JsonKey(name: r'hasSequenceDrift', required: true, includeIfNull: false)
  final bool hasSequenceDrift;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CreditReconciliationWallet &&
          other.accountId == accountId &&
          other.expectedBalance == expectedBalance &&
          other.actualBalance == actualBalance &&
          other.delta == delta &&
          other.expectedLastSeq == expectedLastSeq &&
          other.actualLastSeq == actualLastSeq &&
          other.walletExists == walletExists &&
          other.hasBalanceDrift == hasBalanceDrift &&
          other.hasSequenceDrift == hasSequenceDrift;

  @override
  int get hashCode =>
      accountId.hashCode +
      expectedBalance.hashCode +
      (actualBalance == null ? 0 : actualBalance.hashCode) +
      delta.hashCode +
      expectedLastSeq.hashCode +
      (actualLastSeq == null ? 0 : actualLastSeq.hashCode) +
      walletExists.hashCode +
      hasBalanceDrift.hashCode +
      hasSequenceDrift.hashCode;

  factory CreditReconciliationWallet.fromJson(Map<String, dynamic> json) =>
      _$CreditReconciliationWalletFromJson(json);

  Map<String, dynamic> toJson() => _$CreditReconciliationWalletToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
