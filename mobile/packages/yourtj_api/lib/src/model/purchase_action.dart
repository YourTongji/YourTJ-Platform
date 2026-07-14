//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'purchase_action.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PurchaseAction {
  /// Returns a new [PurchaseAction] instance.
  PurchaseAction({required this.action});

  @JsonKey(
    name: r'action',
    required: true,
    includeIfNull: false,
    unknownEnumValue: PurchaseActionActionEnum.unknownDefaultOpenApi,
  )
  final PurchaseActionActionEnum action;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PurchaseAction && other.action == action;

  @override
  int get hashCode => action.hashCode;

  factory PurchaseAction.fromJson(Map<String, dynamic> json) =>
      _$PurchaseActionFromJson(json);

  Map<String, dynamic> toJson() => _$PurchaseActionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum PurchaseActionActionEnum {
  @JsonValue(r'accept')
  accept(r'accept'),
  @JsonValue(r'deliver')
  deliver(r'deliver'),
  @JsonValue(r'confirm')
  confirm(r'confirm'),
  @JsonValue(r'cancel')
  cancel(r'cancel'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const PurchaseActionActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
