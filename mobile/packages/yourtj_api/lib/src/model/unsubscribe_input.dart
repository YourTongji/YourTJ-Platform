//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'unsubscribe_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UnsubscribeInput {
  /// Returns a new [UnsubscribeInput] instance.
  UnsubscribeInput({required this.targetType, required this.targetId});

  @JsonKey(
    name: r'targetType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: UnsubscribeInputTargetTypeEnum.unknownDefaultOpenApi,
  )
  final UnsubscribeInputTargetTypeEnum targetType;

  @JsonKey(name: r'targetId', required: true, includeIfNull: false)
  final String targetId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UnsubscribeInput &&
          other.targetType == targetType &&
          other.targetId == targetId;

  @override
  int get hashCode => targetType.hashCode + targetId.hashCode;

  factory UnsubscribeInput.fromJson(Map<String, dynamic> json) =>
      _$UnsubscribeInputFromJson(json);

  Map<String, dynamic> toJson() => _$UnsubscribeInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum UnsubscribeInputTargetTypeEnum {
  @JsonValue(r'board')
  board(r'board'),
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const UnsubscribeInputTargetTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
