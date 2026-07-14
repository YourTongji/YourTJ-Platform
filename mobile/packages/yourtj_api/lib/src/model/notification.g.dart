// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notification.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Notification _$NotificationFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Notification', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'type',
          'payload',
          'targetUrl',
          'read',
          'readAt',
          'createdAt',
        ],
      );
      final val = Notification(
        id: $checkedConvert('id', (v) => v as String),
        type: $checkedConvert('type', (v) => v as String),
        payload: $checkedConvert(
          'payload',
          (v) => (v as Map<String, dynamic>).map(
            (k, e) => MapEntry(k, e as Object),
          ),
        ),
        targetUrl: $checkedConvert('targetUrl', (v) => v as String?),
        read: $checkedConvert('read', (v) => v as bool),
        readAt: $checkedConvert('readAt', (v) => (v as num?)?.toInt()),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$NotificationToJson(Notification instance) =>
    <String, dynamic>{
      'id': instance.id,
      'type': instance.type,
      'payload': instance.payload,
      'targetUrl': instance.targetUrl,
      'read': instance.read,
      'readAt': instance.readAt,
      'createdAt': instance.createdAt,
    };
