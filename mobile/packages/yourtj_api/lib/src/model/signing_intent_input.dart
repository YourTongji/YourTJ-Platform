//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'signing_intent_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SigningIntentInput {
  /// Returns a new [SigningIntentInput] instance.
  SigningIntentInput({required this.action, required this.request});

  @JsonKey(
    name: r'action',
    required: true,
    includeIfNull: false,
    unknownEnumValue: SigningIntentInputActionEnum.unknownDefaultOpenApi,
  )
  final SigningIntentInputActionEnum action;

  /// Action-specific JSON object whose exact normalized content is bound into requestHash.
  @JsonKey(name: r'request', required: true, includeIfNull: false)
  final Map<String, Object> request;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SigningIntentInput &&
          other.action == action &&
          other.request == request;

  @override
  int get hashCode => action.hashCode + request.hashCode;

  factory SigningIntentInput.fromJson(Map<String, dynamic> json) =>
      _$SigningIntentInputFromJson(json);

  Map<String, dynamic> toJson() => _$SigningIntentInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum SigningIntentInputActionEnum {
  @JsonValue(r'credit.tip')
  creditPeriodTip(r'credit.tip'),
  @JsonValue(r'credit.task.create')
  creditPeriodTaskPeriodCreate(r'credit.task.create'),
  @JsonValue(r'credit.task.action')
  creditPeriodTaskPeriodAction(r'credit.task.action'),
  @JsonValue(r'credit.product.purchase')
  creditPeriodProductPeriodPurchase(r'credit.product.purchase'),
  @JsonValue(r'credit.purchase.action')
  creditPeriodPurchasePeriodAction(r'credit.purchase.action'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SigningIntentInputActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
