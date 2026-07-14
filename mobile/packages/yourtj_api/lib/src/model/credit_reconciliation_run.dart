//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'credit_reconciliation_run.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CreditReconciliationRun {
  /// Returns a new [CreditReconciliationRun] instance.
  CreditReconciliationRun({
    required this.id,

    required this.status,

    required this.requestedBy,

    required this.reason,

    this.ledgerOk,

    this.ledgerLatestSeq,

    this.ledgerLatestHash,

    this.ledgerFailureSeq,

    required this.walletsChecked,

    required this.driftedWallets,

    required this.missingWallets,

    required this.balanceDriftedWallets,

    required this.sequenceDriftedWallets,

    required this.totalAbsoluteDrift,

    this.errorCode,

    required this.createdAt,

    this.startedAt,

    this.completedAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: CreditReconciliationRunStatusEnum.unknownDefaultOpenApi,
  )
  final CreditReconciliationRunStatusEnum status;

  @JsonKey(name: r'requestedBy', required: true, includeIfNull: false)
  final String requestedBy;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @JsonKey(name: r'ledgerOk', required: false, includeIfNull: false)
  final bool? ledgerOk;

  @JsonKey(name: r'ledgerLatestSeq', required: false, includeIfNull: false)
  final int? ledgerLatestSeq;

  @JsonKey(name: r'ledgerLatestHash', required: false, includeIfNull: false)
  final String? ledgerLatestHash;

  @JsonKey(name: r'ledgerFailureSeq', required: false, includeIfNull: false)
  final int? ledgerFailureSeq;

  // minimum: 0
  @JsonKey(name: r'walletsChecked', required: true, includeIfNull: false)
  final int walletsChecked;

  // minimum: 0
  @JsonKey(name: r'driftedWallets', required: true, includeIfNull: false)
  final int driftedWallets;

  // minimum: 0
  @JsonKey(name: r'missingWallets', required: true, includeIfNull: false)
  final int missingWallets;

  // minimum: 0
  @JsonKey(name: r'balanceDriftedWallets', required: true, includeIfNull: false)
  final int balanceDriftedWallets;

  // minimum: 0
  @JsonKey(
    name: r'sequenceDriftedWallets',
    required: true,
    includeIfNull: false,
  )
  final int sequenceDriftedWallets;

  @JsonKey(name: r'totalAbsoluteDrift', required: true, includeIfNull: false)
  final String totalAbsoluteDrift;

  @JsonKey(name: r'errorCode', required: false, includeIfNull: false)
  final String? errorCode;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @JsonKey(name: r'startedAt', required: false, includeIfNull: false)
  final int? startedAt;

  @JsonKey(name: r'completedAt', required: false, includeIfNull: false)
  final int? completedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CreditReconciliationRun &&
          other.id == id &&
          other.status == status &&
          other.requestedBy == requestedBy &&
          other.reason == reason &&
          other.ledgerOk == ledgerOk &&
          other.ledgerLatestSeq == ledgerLatestSeq &&
          other.ledgerLatestHash == ledgerLatestHash &&
          other.ledgerFailureSeq == ledgerFailureSeq &&
          other.walletsChecked == walletsChecked &&
          other.driftedWallets == driftedWallets &&
          other.missingWallets == missingWallets &&
          other.balanceDriftedWallets == balanceDriftedWallets &&
          other.sequenceDriftedWallets == sequenceDriftedWallets &&
          other.totalAbsoluteDrift == totalAbsoluteDrift &&
          other.errorCode == errorCode &&
          other.createdAt == createdAt &&
          other.startedAt == startedAt &&
          other.completedAt == completedAt;

  @override
  int get hashCode =>
      id.hashCode +
      status.hashCode +
      requestedBy.hashCode +
      reason.hashCode +
      (ledgerOk == null ? 0 : ledgerOk.hashCode) +
      (ledgerLatestSeq == null ? 0 : ledgerLatestSeq.hashCode) +
      (ledgerLatestHash == null ? 0 : ledgerLatestHash.hashCode) +
      (ledgerFailureSeq == null ? 0 : ledgerFailureSeq.hashCode) +
      walletsChecked.hashCode +
      driftedWallets.hashCode +
      missingWallets.hashCode +
      balanceDriftedWallets.hashCode +
      sequenceDriftedWallets.hashCode +
      totalAbsoluteDrift.hashCode +
      (errorCode == null ? 0 : errorCode.hashCode) +
      createdAt.hashCode +
      (startedAt == null ? 0 : startedAt.hashCode) +
      (completedAt == null ? 0 : completedAt.hashCode);

  factory CreditReconciliationRun.fromJson(Map<String, dynamic> json) =>
      _$CreditReconciliationRunFromJson(json);

  Map<String, dynamic> toJson() => _$CreditReconciliationRunToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum CreditReconciliationRunStatusEnum {
  @JsonValue(r'queued')
  queued(r'queued'),
  @JsonValue(r'running')
  running(r'running'),
  @JsonValue(r'succeeded')
  succeeded(r'succeeded'),
  @JsonValue(r'failed')
  failed(r'failed'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const CreditReconciliationRunStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
