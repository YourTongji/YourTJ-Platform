// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task_action.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TaskAction _$TaskActionFromJson(Map<String, dynamic> json) =>
    $checkedCreate('TaskAction', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['action']);
      final val = TaskAction(
        action: $checkedConvert(
          'action',
          (v) => $enumDecode(
            _$TaskActionActionEnumEnumMap,
            v,
            unknownValue: TaskActionActionEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$TaskActionToJson(TaskAction instance) =>
    <String, dynamic>{
      'action': _$TaskActionActionEnumEnumMap[instance.action]!,
    };

const _$TaskActionActionEnumEnumMap = {
  TaskActionActionEnum.submit: 'submit',
  TaskActionActionEnum.confirm: 'confirm',
  TaskActionActionEnum.cancel: 'cancel',
  TaskActionActionEnum.reject: 'reject',
  TaskActionActionEnum.delete: 'delete',
  TaskActionActionEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
