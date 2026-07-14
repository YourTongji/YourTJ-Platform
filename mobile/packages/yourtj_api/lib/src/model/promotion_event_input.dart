//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'promotion_event_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PromotionEventInput {
  /// Returns a new [PromotionEventInput] instance.
  PromotionEventInput({required this.eventType, required this.trackingToken});

  @JsonKey(
    name: r'eventType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PromotionEventInputEventTypeEnum.unknownDefaultOpenApi,
  )
  final PromotionEventInputEventTypeEnum eventType;

  @JsonKey(name: r'trackingToken', required: true, includeIfNull: false)
  final String trackingToken;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PromotionEventInput &&
          other.eventType == eventType &&
          other.trackingToken == trackingToken;

  @override
  int get hashCode => eventType.hashCode + trackingToken.hashCode;

  factory PromotionEventInput.fromJson(Map<String, dynamic> json) =>
      _$PromotionEventInputFromJson(json);

  Map<String, dynamic> toJson() => _$PromotionEventInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum PromotionEventInputEventTypeEnum {
  @JsonValue(r'impression')
  impression(r'impression'),
  @JsonValue(r'click')
  click(r'click'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PromotionEventInputEventTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
