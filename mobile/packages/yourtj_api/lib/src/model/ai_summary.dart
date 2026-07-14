//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'ai_summary.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AiSummary {
  /// Returns a new [AiSummary] instance.
  AiSummary({this.courseId, this.summary, this.model, this.updatedAt});

  @JsonKey(name: r'courseId', required: false, includeIfNull: false)
  final String? courseId;

  @JsonKey(name: r'summary', required: false, includeIfNull: false)
  final String? summary;

  @JsonKey(name: r'model', required: false, includeIfNull: false)
  final String? model;

  @JsonKey(name: r'updatedAt', required: false, includeIfNull: false)
  final int? updatedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AiSummary &&
          other.courseId == courseId &&
          other.summary == summary &&
          other.model == model &&
          other.updatedAt == updatedAt;

  @override
  int get hashCode =>
      courseId.hashCode +
      summary.hashCode +
      model.hashCode +
      updatedAt.hashCode;

  factory AiSummary.fromJson(Map<String, dynamic> json) =>
      _$AiSummaryFromJson(json);

  Map<String, dynamic> toJson() => _$AiSummaryToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
