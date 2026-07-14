//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'ledger_entry.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class LedgerEntry {
  /// Returns a new [LedgerEntry] instance.
  LedgerEntry({
    this.seq,

    this.txId,

    this.type,

    this.fromAccount,

    this.toAccount,

    this.amount,

    this.hash,

    this.createdAt,
  });

  @JsonKey(name: r'seq', required: false, includeIfNull: false)
  final int? seq;

  @JsonKey(name: r'txId', required: false, includeIfNull: false)
  final String? txId;

  @JsonKey(
    name: r'type',
    required: false,
    includeIfNull: false,
    unknownEnumValue: LedgerEntryTypeEnum.unknownDefaultOpenApi,
  )
  final LedgerEntryTypeEnum? type;

  @JsonKey(name: r'fromAccount', required: false, includeIfNull: false)
  final String? fromAccount;

  @JsonKey(name: r'toAccount', required: false, includeIfNull: false)
  final String? toAccount;

  @JsonKey(name: r'amount', required: false, includeIfNull: false)
  final int? amount;

  @JsonKey(name: r'hash', required: false, includeIfNull: false)
  final String? hash;

  @JsonKey(name: r'createdAt', required: false, includeIfNull: false)
  final int? createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is LedgerEntry &&
          other.seq == seq &&
          other.txId == txId &&
          other.type == type &&
          other.fromAccount == fromAccount &&
          other.toAccount == toAccount &&
          other.amount == amount &&
          other.hash == hash &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      seq.hashCode +
      txId.hashCode +
      type.hashCode +
      (fromAccount == null ? 0 : fromAccount.hashCode) +
      (toAccount == null ? 0 : toAccount.hashCode) +
      amount.hashCode +
      hash.hashCode +
      createdAt.hashCode;

  factory LedgerEntry.fromJson(Map<String, dynamic> json) =>
      _$LedgerEntryFromJson(json);

  Map<String, dynamic> toJson() => _$LedgerEntryToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum LedgerEntryTypeEnum {
  @JsonValue(r'mint')
  mint(r'mint'),
  @JsonValue(r'tip')
  tip(r'tip'),
  @JsonValue(r'escrow_hold')
  escrowHold(r'escrow_hold'),
  @JsonValue(r'escrow_release')
  escrowRelease(r'escrow_release'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const LedgerEntryTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
