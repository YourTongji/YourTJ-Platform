// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notification_preferences_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

NotificationPreferencesInput _$NotificationPreferencesInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('NotificationPreferencesInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['inApp', 'email']);
  final val = NotificationPreferencesInput(
    inApp: $checkedConvert(
      'inApp',
      (v) => InAppNotificationPrefsInput.fromJson(v as Map<String, dynamic>),
    ),
    email: $checkedConvert(
      'email',
      (v) => EmailNotificationPrefs.fromJson(v as Map<String, dynamic>),
    ),
  );
  return val;
});

Map<String, dynamic> _$NotificationPreferencesInputToJson(
  NotificationPreferencesInput instance,
) => <String, dynamic>{
  'inApp': instance.inApp.toJson(),
  'email': instance.email.toJson(),
};
