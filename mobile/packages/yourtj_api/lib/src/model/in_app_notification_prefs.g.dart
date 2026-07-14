// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'in_app_notification_prefs.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

InAppNotificationPrefs _$InAppNotificationPrefsFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('InAppNotificationPrefs', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'replies',
      'mentions',
      'quotes',
      'votes',
      'badges',
      'follows',
      'subscriptions',
      'directMessages',
    ],
  );
  final val = InAppNotificationPrefs(
    replies: $checkedConvert('replies', (v) => v as bool? ?? true),
    mentions: $checkedConvert('mentions', (v) => v as bool? ?? true),
    quotes: $checkedConvert('quotes', (v) => v as bool? ?? true),
    votes: $checkedConvert('votes', (v) => v as bool? ?? true),
    badges: $checkedConvert('badges', (v) => v as bool? ?? true),
    follows: $checkedConvert('follows', (v) => v as bool? ?? true),
    subscriptions: $checkedConvert('subscriptions', (v) => v as bool? ?? true),
    directMessages: $checkedConvert(
      'directMessages',
      (v) => v as bool? ?? true,
    ),
  );
  return val;
});

Map<String, dynamic> _$InAppNotificationPrefsToJson(
  InAppNotificationPrefs instance,
) => <String, dynamic>{
  'replies': instance.replies,
  'mentions': instance.mentions,
  'quotes': instance.quotes,
  'votes': instance.votes,
  'badges': instance.badges,
  'follows': instance.follows,
  'subscriptions': instance.subscriptions,
  'directMessages': instance.directMessages,
};
