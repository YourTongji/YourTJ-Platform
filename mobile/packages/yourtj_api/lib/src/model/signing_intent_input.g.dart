// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'signing_intent_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SigningIntentInput _$SigningIntentInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('SigningIntentInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['action', 'request']);
      final val = SigningIntentInput(
        action: $checkedConvert(
          'action',
          (v) => $enumDecode(
            _$SigningIntentInputActionEnumEnumMap,
            v,
            unknownValue: SigningIntentInputActionEnum.unknownDefaultOpenApi,
          ),
        ),
        request: $checkedConvert(
          'request',
          (v) => (v as Map<String, dynamic>).map(
            (k, e) => MapEntry(k, e as Object),
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$SigningIntentInputToJson(SigningIntentInput instance) =>
    <String, dynamic>{
      'action': _$SigningIntentInputActionEnumEnumMap[instance.action]!,
      'request': instance.request,
    };

const _$SigningIntentInputActionEnumEnumMap = {
  SigningIntentInputActionEnum.creditPeriodTip: 'credit.tip',
  SigningIntentInputActionEnum.creditPeriodTaskPeriodCreate:
      'credit.task.create',
  SigningIntentInputActionEnum.creditPeriodTaskPeriodAction:
      'credit.task.action',
  SigningIntentInputActionEnum.creditPeriodProductPeriodPurchase:
      'credit.product.purchase',
  SigningIntentInputActionEnum.creditPeriodPurchasePeriodAction:
      'credit.purchase.action',
  SigningIntentInputActionEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
