// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notification_prefs.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

NotificationPrefs _$NotificationPrefsFromJson(Map<String, dynamic> json) =>
    $checkedCreate('NotificationPrefs', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['prefs']);
      final val = NotificationPrefs(
        prefs: $checkedConvert(
          'prefs',
          (v) => NotificationPreferences.fromJson(v as Map<String, dynamic>),
        ),
      );
      return val;
    });

Map<String, dynamic> _$NotificationPrefsToJson(NotificationPrefs instance) =>
    <String, dynamic>{'prefs': instance.prefs.toJson()};
