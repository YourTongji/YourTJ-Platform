//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'reconciliation_run_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ReconciliationRunInput {
  /// Returns a new [ReconciliationRunInput] instance.
  ReconciliationRunInput({required this.reason});

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ReconciliationRunInput && other.reason == reason;

  @override
  int get hashCode => reason.hashCode;

  factory ReconciliationRunInput.fromJson(Map<String, dynamic> json) =>
      _$ReconciliationRunInputFromJson(json);

  Map<String, dynamic> toJson() => _$ReconciliationRunInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
