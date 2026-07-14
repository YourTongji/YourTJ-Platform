//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/media_reconciliation_issue_code.dart';
import 'package:json_annotation/json_annotation.dart';

part 'media_reconciliation_finding.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MediaReconciliationFinding {
  /// Returns a new [MediaReconciliationFinding] instance.
  MediaReconciliationFinding({required this.assetId, required this.issueCodes});

  @JsonKey(name: r'assetId', required: true, includeIfNull: false)
  final String assetId;

  @JsonKey(name: r'issueCodes', required: true, includeIfNull: false)
  final Set<MediaReconciliationIssueCode> issueCodes;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MediaReconciliationFinding &&
          other.assetId == assetId &&
          other.issueCodes == issueCodes;

  @override
  int get hashCode => assetId.hashCode + issueCodes.hashCode;

  factory MediaReconciliationFinding.fromJson(Map<String, dynamic> json) =>
      _$MediaReconciliationFindingFromJson(json);

  Map<String, dynamic> toJson() => _$MediaReconciliationFindingToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
