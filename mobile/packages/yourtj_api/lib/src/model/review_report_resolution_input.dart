//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'review_report_resolution_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ReviewReportResolutionInput {
  /// Returns a new [ReviewReportResolutionInput] instance.
  ReviewReportResolutionInput({required this.action, required this.note});

  @JsonKey(
    name: r'action',
    required: true,
    includeIfNull: false,
    unknownEnumValue:
        ReviewReportResolutionInputActionEnum.unknownDefaultOpenApi,
  )
  final ReviewReportResolutionInputActionEnum action;

  @JsonKey(name: r'note', required: true, includeIfNull: false)
  final String note;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ReviewReportResolutionInput &&
          other.action == action &&
          other.note == note;

  @override
  int get hashCode => action.hashCode + note.hashCode;

  factory ReviewReportResolutionInput.fromJson(Map<String, dynamic> json) =>
      _$ReviewReportResolutionInputFromJson(json);

  Map<String, dynamic> toJson() => _$ReviewReportResolutionInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum ReviewReportResolutionInputActionEnum {
  @JsonValue(r'uphold')
  uphold(r'uphold'),
  @JsonValue(r'reject')
  reject(r'reject'),
  @JsonValue(r'ignore')
  ignore(r'ignore'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const ReviewReportResolutionInputActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
