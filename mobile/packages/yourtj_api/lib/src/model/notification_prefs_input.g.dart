// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notification_prefs_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

NotificationPrefsInput _$NotificationPrefsInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('NotificationPrefsInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['prefs']);
  final val = NotificationPrefsInput(
    prefs: $checkedConvert(
      'prefs',
      (v) => NotificationPreferencesInput.fromJson(v as Map<String, dynamic>),
    ),
  );
  return val;
});

Map<String, dynamic> _$NotificationPrefsInputToJson(
  NotificationPrefsInput instance,
) => <String, dynamic>{'prefs': instance.prefs.toJson()};
