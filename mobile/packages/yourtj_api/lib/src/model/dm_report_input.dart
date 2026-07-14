//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'dm_report_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmReportInput {
  /// Returns a new [DmReportInput] instance.
  DmReportInput({required this.reason, this.note});

  @JsonKey(
    name: r'reason',
    required: true,
    includeIfNull: false,
    unknownEnumValue: DmReportInputReasonEnum.unknownDefaultOpenApi,
  )
  final DmReportInputReasonEnum reason;

  @JsonKey(name: r'note', required: false, includeIfNull: false)
  final String? note;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmReportInput && other.reason == reason && other.note == note;

  @override
  int get hashCode => reason.hashCode + note.hashCode;

  factory DmReportInput.fromJson(Map<String, dynamic> json) =>
      _$DmReportInputFromJson(json);

  Map<String, dynamic> toJson() => _$DmReportInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum DmReportInputReasonEnum {
  @JsonValue(r'spam')
  spam(r'spam'),
  @JsonValue(r'abuse')
  abuse(r'abuse'),
  @JsonValue(r'harassment')
  harassment(r'harassment'),
  @JsonValue(r'fraud')
  fraud(r'fraud'),
  @JsonValue(r'illegal')
  illegal(r'illegal'),
  @JsonValue(r'other')
  other(r'other'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DmReportInputReasonEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
