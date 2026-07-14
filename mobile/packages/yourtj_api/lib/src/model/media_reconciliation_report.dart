//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/media_reconciliation_finding.dart';
import 'package:yourtj_api/src/model/media_provider_inventory_status.dart';
import 'package:json_annotation/json_annotation.dart';

part 'media_reconciliation_report.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaReconciliationReport {
  /// Returns a new [MediaReconciliationReport] instance.
  MediaReconciliationReport({
    required this.dryRun,

    required this.items,

    required this.nextCursor,

    required this.providerInventory,
  });

  /// Always true; this endpoint never repairs findings.
  @JsonKey(name: r'dryRun', required: true, includeIfNull: false)
  final bool dryRun;

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<MediaReconciliationFinding> items;

  /// Last asset id when another bounded page of findings exists.
  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'providerInventory', required: true, includeIfNull: false)
  final MediaProviderInventoryStatus providerInventory;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaReconciliationReport &&
          other.dryRun == dryRun &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.providerInventory == providerInventory;

  @override
  int get hashCode =>
      dryRun.hashCode +
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      providerInventory.hashCode;

  factory MediaReconciliationReport.fromJson(Map<String, dynamic> json) =>
      _$MediaReconciliationReportFromJson(json);

  Map<String, dynamic> toJson() => _$MediaReconciliationReportToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
