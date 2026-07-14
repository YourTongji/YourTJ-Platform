//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'dm_report_resolution_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmReportResolutionInput {
  /// Returns a new [DmReportResolutionInput] instance.
  DmReportResolutionInput({required this.action, this.note});

  @JsonKey(
    name: r'action',
    required: true,
    includeIfNull: false,
    unknownEnumValue: DmReportResolutionInputActionEnum.unknownDefaultOpenApi,
  )
  final DmReportResolutionInputActionEnum action;

  @JsonKey(name: r'note', required: false, includeIfNull: false)
  final String? note;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmReportResolutionInput &&
          other.action == action &&
          other.note == note;

  @override
  int get hashCode => action.hashCode + note.hashCode;

  factory DmReportResolutionInput.fromJson(Map<String, dynamic> json) =>
      _$DmReportResolutionInputFromJson(json);

  Map<String, dynamic> toJson() => _$DmReportResolutionInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum DmReportResolutionInputActionEnum {
  @JsonValue(r'uphold')
  uphold(r'uphold'),
  @JsonValue(r'reject')
  reject(r'reject'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DmReportResolutionInputActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
