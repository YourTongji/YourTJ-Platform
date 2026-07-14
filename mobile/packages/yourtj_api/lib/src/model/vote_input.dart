//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'vote_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class VoteInput {
  /// Returns a new [VoteInput] instance.
  VoteInput({required this.value, required this.postType});

  @JsonKey(
    name: r'value',
    required: true,
    includeIfNull: false,
    unknownEnumValue: VoteInputValueEnum.unknownDefaultOpenApi,
  )
  final VoteInputValueEnum value;

  @JsonKey(
    name: r'postType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: VoteInputPostTypeEnum.unknownDefaultOpenApi,
  )
  final VoteInputPostTypeEnum postType;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is VoteInput && other.value == value && other.postType == postType;

  @override
  int get hashCode => value.hashCode + postType.hashCode;

  factory VoteInput.fromJson(Map<String, dynamic> json) =>
      _$VoteInputFromJson(json);

  Map<String, dynamic> toJson() => _$VoteInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum VoteInputValueEnum {
  @JsonValue(r'up')
  up(r'up'),
  @JsonValue(r'down')
  down(r'down'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const VoteInputValueEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum VoteInputPostTypeEnum {
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'comment')
  comment(r'comment'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const VoteInputPostTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
