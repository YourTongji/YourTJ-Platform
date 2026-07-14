//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'flag_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class FlagInput {
  /// Returns a new [FlagInput] instance.
  FlagInput({required this.reason, this.note, required this.postType});

  @JsonKey(
    name: r'reason',
    required: true,
    includeIfNull: false,
    unknownEnumValue: FlagInputReasonEnum.unknownDefaultOpenApi,
  )
  final FlagInputReasonEnum reason;

  @JsonKey(name: r'note', required: false, includeIfNull: false)
  final String? note;

  @JsonKey(
    name: r'postType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: FlagInputPostTypeEnum.unknownDefaultOpenApi,
  )
  final FlagInputPostTypeEnum postType;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is FlagInput &&
          other.reason == reason &&
          other.note == note &&
          other.postType == postType;

  @override
  int get hashCode => reason.hashCode + note.hashCode + postType.hashCode;

  factory FlagInput.fromJson(Map<String, dynamic> json) =>
      _$FlagInputFromJson(json);

  Map<String, dynamic> toJson() => _$FlagInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum FlagInputReasonEnum {
  @JsonValue(r'spam')
  spam(r'spam'),
  @JsonValue(r'abuse')
  abuse(r'abuse'),
  @JsonValue(r'off_topic')
  offTopic(r'off_topic'),
  @JsonValue(r'illegal')
  illegal(r'illegal'),
  @JsonValue(r'other')
  other(r'other'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const FlagInputReasonEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum FlagInputPostTypeEnum {
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'comment')
  comment(r'comment'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const FlagInputPostTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
