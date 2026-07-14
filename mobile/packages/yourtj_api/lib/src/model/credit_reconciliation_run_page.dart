//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/credit_reconciliation_run.dart';
import 'package:json_annotation/json_annotation.dart';

part 'credit_reconciliation_run_page.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CreditReconciliationRunPage {
  /// Returns a new [CreditReconciliationRunPage] instance.
  CreditReconciliationRunPage({
    required this.items,

    required this.nextCursor,

    required this.hasMore,
  });

  @JsonKey(name: r'items', required: true, includeIfNull: false)
  final List<CreditReconciliationRun> items;

  @JsonKey(name: r'nextCursor', required: true, includeIfNull: true)
  final String? nextCursor;

  @JsonKey(name: r'hasMore', required: true, includeIfNull: false)
  final bool hasMore;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CreditReconciliationRunPage &&
          other.items == items &&
          other.nextCursor == nextCursor &&
          other.hasMore == hasMore;

  @override
  int get hashCode =>
      items.hashCode +
      (nextCursor == null ? 0 : nextCursor.hashCode) +
      hasMore.hashCode;

  factory CreditReconciliationRunPage.fromJson(Map<String, dynamic> json) =>
      _$CreditReconciliationRunPageFromJson(json);

  Map<String, dynamic> toJson() => _$CreditReconciliationRunPageToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
