// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TaskInput _$TaskInputFromJson(Map<String, dynamic> json) => $checkedCreate(
  'TaskInput',
  json,
  ($checkedConvert) {
    $checkKeys(json, requiredKeys: const ['title', 'rewardAmount']);
    final val = TaskInput(
      title: $checkedConvert('title', (v) => v as String),
      description: $checkedConvert('description', (v) => v as String?),
      rewardAmount: $checkedConvert('rewardAmount', (v) => (v as num).toInt()),
      contactInfo: $checkedConvert('contactInfo', (v) => v as String?),
    );
    return val;
  },
);

Map<String, dynamic> _$TaskInputToJson(TaskInput instance) => <String, dynamic>{
  'title': instance.title,
  'description': ?instance.description,
  'rewardAmount': instance.rewardAmount,
  'contactInfo': ?instance.contactInfo,
};
