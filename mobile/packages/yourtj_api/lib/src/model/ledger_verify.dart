//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'ledger_verify.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class LedgerVerify {
  /// Returns a new [LedgerVerify] instance.
  LedgerVerify({this.ok, this.latestSeq, this.latestHash});

  @JsonKey(name: r'ok', required: false, includeIfNull: false)
  final bool? ok;

  @JsonKey(name: r'latestSeq', required: false, includeIfNull: false)
  final int? latestSeq;

  @JsonKey(name: r'latestHash', required: false, includeIfNull: false)
  final String? latestHash;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is LedgerVerify &&
          other.ok == ok &&
          other.latestSeq == latestSeq &&
          other.latestHash == latestHash;

  @override
  int get hashCode => ok.hashCode + latestSeq.hashCode + latestHash.hashCode;

  factory LedgerVerify.fromJson(Map<String, dynamic> json) =>
      _$LedgerVerifyFromJson(json);

  Map<String, dynamic> toJson() => _$LedgerVerifyToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
