// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notification_outbox_event.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

NotificationOutboxEvent _$NotificationOutboxEventFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('NotificationOutboxEvent', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'topic',
      'recipientAccountId',
      'eventType',
      'state',
      'attempts',
      'maxAttempts',
      'manualRetryCount',
      'availableAt',
      'createdAt',
      'updatedAt',
    ],
  );
  final val = NotificationOutboxEvent(
    id: $checkedConvert('id', (v) => v as String),
    topic: $checkedConvert(
      'topic',
      (v) => $enumDecode(
        _$NotificationOutboxEventTopicEnumEnumMap,
        v,
        unknownValue: NotificationOutboxEventTopicEnum.unknownDefaultOpenApi,
      ),
    ),
    recipientAccountId: $checkedConvert(
      'recipientAccountId',
      (v) => v as String,
    ),
    eventType: $checkedConvert('eventType', (v) => v as String),
    state: $checkedConvert(
      'state',
      (v) => $enumDecode(
        _$NotificationOutboxEventStateEnumEnumMap,
        v,
        unknownValue: NotificationOutboxEventStateEnum.unknownDefaultOpenApi,
      ),
    ),
    attempts: $checkedConvert('attempts', (v) => (v as num).toInt()),
    maxAttempts: $checkedConvert('maxAttempts', (v) => (v as num).toInt()),
    manualRetryCount: $checkedConvert(
      'manualRetryCount',
      (v) => (v as num).toInt(),
    ),
    availableAt: $checkedConvert('availableAt', (v) => (v as num).toInt()),
    lastErrorCode: $checkedConvert('lastErrorCode', (v) => v as String?),
    completedAt: $checkedConvert('completedAt', (v) => (v as num?)?.toInt()),
    deadAt: $checkedConvert('deadAt', (v) => (v as num?)?.toInt()),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
    updatedAt: $checkedConvert('updatedAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$NotificationOutboxEventToJson(
  NotificationOutboxEvent instance,
) => <String, dynamic>{
  'id': instance.id,
  'topic': _$NotificationOutboxEventTopicEnumEnumMap[instance.topic]!,
  'recipientAccountId': instance.recipientAccountId,
  'eventType': instance.eventType,
  'state': _$NotificationOutboxEventStateEnumEnumMap[instance.state]!,
  'attempts': instance.attempts,
  'maxAttempts': instance.maxAttempts,
  'manualRetryCount': instance.manualRetryCount,
  'availableAt': instance.availableAt,
  'lastErrorCode': ?instance.lastErrorCode,
  'completedAt': ?instance.completedAt,
  'deadAt': ?instance.deadAt,
  'createdAt': instance.createdAt,
  'updatedAt': instance.updatedAt,
};

const _$NotificationOutboxEventTopicEnumEnumMap = {
  NotificationOutboxEventTopicEnum.notification: 'notification',
  NotificationOutboxEventTopicEnum.achievementAward: 'achievement_award',
  NotificationOutboxEventTopicEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$NotificationOutboxEventStateEnumEnumMap = {
  NotificationOutboxEventStateEnum.queued: 'queued',
  NotificationOutboxEventStateEnum.running: 'running',
  NotificationOutboxEventStateEnum.succeeded: 'succeeded',
  NotificationOutboxEventStateEnum.dead: 'dead',
  NotificationOutboxEventStateEnum.cancelled: 'cancelled',
  NotificationOutboxEventStateEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
