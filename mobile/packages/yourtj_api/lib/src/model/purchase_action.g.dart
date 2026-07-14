// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'purchase_action.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PurchaseAction _$PurchaseActionFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PurchaseAction', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['action']);
      final val = PurchaseAction(
        action: $checkedConvert(
          'action',
          (v) => $enumDecode(
            _$PurchaseActionActionEnumEnumMap,
            v,
            unknownValue: PurchaseActionActionEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$PurchaseActionToJson(PurchaseAction instance) =>
    <String, dynamic>{
      'action': _$PurchaseActionActionEnumEnumMap[instance.action]!,
    };

const _$PurchaseActionActionEnumEnumMap = {
  PurchaseActionActionEnum.accept: 'accept',
  PurchaseActionActionEnum.deliver: 'deliver',
  PurchaseActionActionEnum.confirm: 'confirm',
  PurchaseActionActionEnum.cancel: 'cancel',
  PurchaseActionActionEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
