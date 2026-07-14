// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'delete_account_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DeleteAccountInput _$DeleteAccountInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DeleteAccountInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['confirmation']);
      final val = DeleteAccountInput(
        confirmation: $checkedConvert(
          'confirmation',
          (v) => $enumDecode(
            _$DeleteAccountInputConfirmationEnumEnumMap,
            v,
            unknownValue:
                DeleteAccountInputConfirmationEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$DeleteAccountInputToJson(DeleteAccountInput instance) =>
    <String, dynamic>{
      'confirmation':
          _$DeleteAccountInputConfirmationEnumEnumMap[instance.confirmation]!,
    };

const _$DeleteAccountInputConfirmationEnumEnumMap = {
  DeleteAccountInputConfirmationEnum.DELETE: 'DELETE',
  DeleteAccountInputConfirmationEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
