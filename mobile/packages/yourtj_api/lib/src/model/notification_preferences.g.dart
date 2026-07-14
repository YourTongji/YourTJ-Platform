// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notification_preferences.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

NotificationPreferences _$NotificationPreferencesFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('NotificationPreferences', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['inApp', 'email']);
  final val = NotificationPreferences(
    inApp: $checkedConvert(
      'inApp',
      (v) => InAppNotificationPrefs.fromJson(v as Map<String, dynamic>),
    ),
    email: $checkedConvert(
      'email',
      (v) => EmailNotificationPrefs.fromJson(v as Map<String, dynamic>),
    ),
  );
  return val;
});

Map<String, dynamic> _$NotificationPreferencesToJson(
  NotificationPreferences instance,
) => <String, dynamic>{
  'inApp': instance.inApp.toJson(),
  'email': instance.email.toJson(),
};
