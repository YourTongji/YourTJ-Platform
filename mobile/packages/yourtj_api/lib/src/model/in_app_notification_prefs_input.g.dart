// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'in_app_notification_prefs_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

InAppNotificationPrefsInput _$InAppNotificationPrefsInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('InAppNotificationPrefsInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'replies',
      'mentions',
      'quotes',
      'votes',
      'badges',
      'subscriptions',
      'directMessages',
    ],
  );
  final val = InAppNotificationPrefsInput(
    replies: $checkedConvert('replies', (v) => v as bool),
    mentions: $checkedConvert('mentions', (v) => v as bool),
    quotes: $checkedConvert('quotes', (v) => v as bool),
    votes: $checkedConvert('votes', (v) => v as bool),
    badges: $checkedConvert('badges', (v) => v as bool),
    follows: $checkedConvert('follows', (v) => v as bool?),
    subscriptions: $checkedConvert('subscriptions', (v) => v as bool),
    directMessages: $checkedConvert('directMessages', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$InAppNotificationPrefsInputToJson(
  InAppNotificationPrefsInput instance,
) => <String, dynamic>{
  'replies': instance.replies,
  'mentions': instance.mentions,
  'quotes': instance.quotes,
  'votes': instance.votes,
  'badges': instance.badges,
  'follows': ?instance.follows,
  'subscriptions': instance.subscriptions,
  'directMessages': instance.directMessages,
};
