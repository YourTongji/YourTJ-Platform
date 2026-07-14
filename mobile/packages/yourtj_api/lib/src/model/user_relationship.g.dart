// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_relationship.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserRelationship _$UserRelationshipFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UserRelationship', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'isSelf',
          'following',
          'followedBy',
          'muted',
          'blockedByMe',
          'blockedMe',
          'canFollow',
          'canStartConversation',
          'canMention',
        ],
      );
      final val = UserRelationship(
        isSelf: $checkedConvert('isSelf', (v) => v as bool),
        following: $checkedConvert('following', (v) => v as bool),
        followedBy: $checkedConvert('followedBy', (v) => v as bool),
        muted: $checkedConvert('muted', (v) => v as bool),
        blockedByMe: $checkedConvert('blockedByMe', (v) => v as bool),
        blockedMe: $checkedConvert('blockedMe', (v) => v as bool),
        canFollow: $checkedConvert('canFollow', (v) => v as bool),
        canStartConversation: $checkedConvert(
          'canStartConversation',
          (v) => v as bool,
        ),
        canMention: $checkedConvert('canMention', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$UserRelationshipToJson(UserRelationship instance) =>
    <String, dynamic>{
      'isSelf': instance.isSelf,
      'following': instance.following,
      'followedBy': instance.followedBy,
      'muted': instance.muted,
      'blockedByMe': instance.blockedByMe,
      'blockedMe': instance.blockedMe,
      'canFollow': instance.canFollow,
      'canStartConversation': instance.canStartConversation,
      'canMention': instance.canMention,
    };
