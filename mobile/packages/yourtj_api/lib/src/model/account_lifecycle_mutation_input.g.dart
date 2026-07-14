// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'account_lifecycle_mutation_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AccountLifecycleMutationInput _$AccountLifecycleMutationInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AccountLifecycleMutationInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['confirmation']);
  final val = AccountLifecycleMutationInput(
    confirmation: $checkedConvert(
      'confirmation',
      (v) => $enumDecode(
        _$AccountLifecycleMutationInputConfirmationEnumEnumMap,
        v,
        unknownValue:
            AccountLifecycleMutationInputConfirmationEnum.unknownDefaultOpenApi,
      ),
    ),
  );
  return val;
});

Map<String, dynamic> _$AccountLifecycleMutationInputToJson(
  AccountLifecycleMutationInput instance,
) => <String, dynamic>{
  'confirmation':
      _$AccountLifecycleMutationInputConfirmationEnumEnumMap[instance
          .confirmation]!,
};

const _$AccountLifecycleMutationInputConfirmationEnumEnumMap = {
  AccountLifecycleMutationInputConfirmationEnum.DEACTIVATE: 'DEACTIVATE',
  AccountLifecycleMutationInputConfirmationEnum.DELETE: 'DELETE',
  AccountLifecycleMutationInputConfirmationEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
