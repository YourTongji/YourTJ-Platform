// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'email_notification_prefs.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

EmailNotificationPrefs _$EmailNotificationPrefsFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('EmailNotificationPrefs', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['weeklyDigest']);
  final val = EmailNotificationPrefs(
    weeklyDigest: $checkedConvert('weeklyDigest', (v) => v as bool? ?? false),
  );
  return val;
});

Map<String, dynamic> _$EmailNotificationPrefsToJson(
  EmailNotificationPrefs instance,
) => <String, dynamic>{'weeklyDigest': instance.weeklyDigest};
