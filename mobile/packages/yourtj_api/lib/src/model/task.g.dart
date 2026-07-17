// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'task.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Task _$TaskFromJson(Map<String, dynamic> json) => $checkedCreate('Task', json, (
  $checkedConvert,
) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'creatorId',
      'acceptorId',
      'title',
      'description',
      'rewardAmount',
      'contactInfo',
      'status',
      'createdAt',
    ],
  );
  final val = Task(
    id: $checkedConvert('id', (v) => v as String),
    creatorId: $checkedConvert('creatorId', (v) => v as String),
    acceptorId: $checkedConvert('acceptorId', (v) => v as String?),
    title: $checkedConvert('title', (v) => v as String),
    description: $checkedConvert('description', (v) => v as String?),
    rewardAmount: $checkedConvert('rewardAmount', (v) => (v as num).toInt()),
    contactInfo: $checkedConvert('contactInfo', (v) => v as String?),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$TaskStatusEnumEnumMap,
        v,
        unknownValue: TaskStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$TaskToJson(Task instance) => <String, dynamic>{
  'id': instance.id,
  'creatorId': instance.creatorId,
  'acceptorId': instance.acceptorId,
  'title': instance.title,
  'description': instance.description,
  'rewardAmount': instance.rewardAmount,
  'contactInfo': instance.contactInfo,
  'status': _$TaskStatusEnumEnumMap[instance.status]!,
  'createdAt': instance.createdAt,
};

const _$TaskStatusEnumEnumMap = {
  TaskStatusEnum.open: 'open',
  TaskStatusEnum.inProgress: 'in_progress',
  TaskStatusEnum.submitted: 'submitted',
  TaskStatusEnum.completed: 'completed',
  TaskStatusEnum.cancelled: 'cancelled',
  TaskStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
