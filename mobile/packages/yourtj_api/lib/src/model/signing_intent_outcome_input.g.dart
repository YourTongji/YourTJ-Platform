// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'signing_intent_outcome_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SigningIntentOutcomeInput _$SigningIntentOutcomeInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('SigningIntentOutcomeInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['intentId']);
  final val = SigningIntentOutcomeInput(
    intentId: $checkedConvert('intentId', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$SigningIntentOutcomeInputToJson(
  SigningIntentOutcomeInput instance,
) => <String, dynamic>{'intentId': instance.intentId};
