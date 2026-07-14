//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'tip_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class TipInput {
  /// Returns a new [TipInput] instance.
  TipInput({
    required this.toAccountId,

    required this.amount,

    required this.targetType,

    required this.targetId,
  });

  @JsonKey(name: r'toAccountId', required: true, includeIfNull: false)
  final String toAccountId;

  // minimum: 1
  @JsonKey(name: r'amount', required: true, includeIfNull: false)
  final int amount;

  @JsonKey(
    name: r'targetType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: TipInputTargetTypeEnum.unknownDefaultOpenApi,
  )
  final TipInputTargetTypeEnum targetType;

  @JsonKey(name: r'targetId', required: true, includeIfNull: false)
  final String targetId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is TipInput &&
          other.toAccountId == toAccountId &&
          other.amount == amount &&
          other.targetType == targetType &&
          other.targetId == targetId;

  @override
  int get hashCode =>
      toAccountId.hashCode +
      amount.hashCode +
      targetType.hashCode +
      targetId.hashCode;

  factory TipInput.fromJson(Map<String, dynamic> json) =>
      _$TipInputFromJson(json);

  Map<String, dynamic> toJson() => _$TipInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum TipInputTargetTypeEnum {
  @JsonValue(r'review')
  review(r'review'),
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'comment')
  comment(r'comment'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const TipInputTargetTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
