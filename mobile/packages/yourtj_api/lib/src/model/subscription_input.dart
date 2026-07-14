//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'subscription_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SubscriptionInput {
  /// Returns a new [SubscriptionInput] instance.
  SubscriptionInput({
    required this.targetType,

    required this.targetId,

    required this.level,
  });

  @JsonKey(
    name: r'targetType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SubscriptionInputTargetTypeEnum.unknownDefaultOpenApi,
  )
  final SubscriptionInputTargetTypeEnum targetType;

  @JsonKey(name: r'targetId', required: true, includeIfNull: false)
  final String targetId;

  @JsonKey(
    name: r'level',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SubscriptionInputLevelEnum.unknownDefaultOpenApi,
  )
  final SubscriptionInputLevelEnum level;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SubscriptionInput &&
          other.targetType == targetType &&
          other.targetId == targetId &&
          other.level == level;

  @override
  int get hashCode => targetType.hashCode + targetId.hashCode + level.hashCode;

  factory SubscriptionInput.fromJson(Map<String, dynamic> json) =>
      _$SubscriptionInputFromJson(json);

  Map<String, dynamic> toJson() => _$SubscriptionInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum SubscriptionInputTargetTypeEnum {
  @JsonValue(r'board')
  board(r'board'),
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SubscriptionInputTargetTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum SubscriptionInputLevelEnum {
  @JsonValue(r'watching')
  watching(r'watching'),
  @JsonValue(r'tracking')
  tracking(r'tracking'),
  @JsonValue(r'muted')
  muted(r'muted'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SubscriptionInputLevelEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
