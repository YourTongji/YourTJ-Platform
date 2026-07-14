// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notification_unread_count.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

NotificationUnreadCount _$NotificationUnreadCountFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('NotificationUnreadCount', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['count']);
  final val = NotificationUnreadCount(
    count: $checkedConvert('count', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$NotificationUnreadCountToJson(
  NotificationUnreadCount instance,
) => <String, dynamic>{'count': instance.count};
