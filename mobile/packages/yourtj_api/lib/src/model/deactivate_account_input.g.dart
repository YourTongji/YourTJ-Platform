// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deactivate_account_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DeactivateAccountInput _$DeactivateAccountInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('DeactivateAccountInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['confirmation']);
  final val = DeactivateAccountInput(
    confirmation: $checkedConvert(
      'confirmation',
      (v) => $enumDecode(
        _$DeactivateAccountInputConfirmationEnumEnumMap,
        v,
        unknownValue:
            DeactivateAccountInputConfirmationEnum.unknownDefaultOpenApi,
      ),
    ),
  );
  return val;
});

Map<String, dynamic> _$DeactivateAccountInputToJson(
  DeactivateAccountInput instance,
) => <String, dynamic>{
  'confirmation':
      _$DeactivateAccountInputConfirmationEnumEnumMap[instance.confirmation]!,
};

const _$DeactivateAccountInputConfirmationEnumEnumMap = {
  DeactivateAccountInputConfirmationEnum.DEACTIVATE: 'DEACTIVATE',
  DeactivateAccountInputConfirmationEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
