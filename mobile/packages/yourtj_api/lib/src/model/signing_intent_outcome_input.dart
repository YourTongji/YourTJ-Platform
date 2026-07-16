//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'signing_intent_outcome_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class SigningIntentOutcomeInput {
  /// Returns a new [SigningIntentOutcomeInput] instance.
  SigningIntentOutcomeInput({required this.intentId});

  @JsonKey(name: r'intentId', required: true, includeIfNull: false)
  final String intentId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SigningIntentOutcomeInput && other.intentId == intentId;

  @override
  int get hashCode => intentId.hashCode;

  factory SigningIntentOutcomeInput.fromJson(Map<String, dynamic> json) =>
      _$SigningIntentOutcomeInputFromJson(json);

  Map<String, dynamic> toJson() => _$SigningIntentOutcomeInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
