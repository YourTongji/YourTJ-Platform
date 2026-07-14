// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notification_outbox_retry_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

NotificationOutboxRetryInput _$NotificationOutboxRetryInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('NotificationOutboxRetryInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = NotificationOutboxRetryInput(
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$NotificationOutboxRetryInputToJson(
  NotificationOutboxRetryInput instance,
) => <String, dynamic>{'reason': instance.reason};
