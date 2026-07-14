// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'profile_privacy_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ProfilePrivacyUpdateInput _$ProfilePrivacyUpdateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ProfilePrivacyUpdateInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'profileVisibility',
      'followersVisibility',
      'followingVisibility',
      'discoverable',
      'dmPolicy',
    ],
  );
  final val = ProfilePrivacyUpdateInput(
    profileVisibility: $checkedConvert(
      'profileVisibility',
      (v) => $enumDecode(
        _$ProfileVisibilityEnumMap,
        v,
        unknownValue: ProfileVisibility.unknownDefaultOpenApi,
      ),
    ),
    activityVisibility: $checkedConvert(
      'activityVisibility',
      (v) => $enumDecodeNullable(
        _$ActivityVisibilityEnumMap,
        v,
        unknownValue: ActivityVisibility.unknownDefaultOpenApi,
      ),
    ),
    followersVisibility: $checkedConvert(
      'followersVisibility',
      (v) => $enumDecode(
        _$RelationshipListVisibilityEnumMap,
        v,
        unknownValue: RelationshipListVisibility.unknownDefaultOpenApi,
      ),
    ),
    followingVisibility: $checkedConvert(
      'followingVisibility',
      (v) => $enumDecode(
        _$RelationshipListVisibilityEnumMap,
        v,
        unknownValue: RelationshipListVisibility.unknownDefaultOpenApi,
      ),
    ),
    discoverable: $checkedConvert('discoverable', (v) => v as bool),
    dmPolicy: $checkedConvert(
      'dmPolicy',
      (v) => $enumDecode(
        _$DmPolicyEnumMap,
        v,
        unknownValue: DmPolicy.unknownDefaultOpenApi,
      ),
    ),
    mentionPolicy: $checkedConvert(
      'mentionPolicy',
      (v) => $enumDecodeNullable(
        _$MentionPolicyEnumMap,
        v,
        unknownValue: MentionPolicy.unknownDefaultOpenApi,
      ),
    ),
  );
  return val;
});

Map<String, dynamic> _$ProfilePrivacyUpdateInputToJson(
  ProfilePrivacyUpdateInput instance,
) => <String, dynamic>{
  'profileVisibility': _$ProfileVisibilityEnumMap[instance.profileVisibility]!,
  'activityVisibility':
      ?_$ActivityVisibilityEnumMap[instance.activityVisibility],
  'followersVisibility':
      _$RelationshipListVisibilityEnumMap[instance.followersVisibility]!,
  'followingVisibility':
      _$RelationshipListVisibilityEnumMap[instance.followingVisibility]!,
  'discoverable': instance.discoverable,
  'dmPolicy': _$DmPolicyEnumMap[instance.dmPolicy]!,
  'mentionPolicy': ?_$MentionPolicyEnumMap[instance.mentionPolicy],
};

const _$ProfileVisibilityEnumMap = {
  ProfileVisibility.public: 'public',
  ProfileVisibility.campus: 'campus',
  ProfileVisibility.onlyMe: 'only_me',
  ProfileVisibility.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$ActivityVisibilityEnumMap = {
  ActivityVisibility.public: 'public',
  ActivityVisibility.campus: 'campus',
  ActivityVisibility.onlyMe: 'only_me',
  ActivityVisibility.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$RelationshipListVisibilityEnumMap = {
  RelationshipListVisibility.public: 'public',
  RelationshipListVisibility.campus: 'campus',
  RelationshipListVisibility.followers: 'followers',
  RelationshipListVisibility.onlyMe: 'only_me',
  RelationshipListVisibility.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$DmPolicyEnumMap = {
  DmPolicy.everyone: 'everyone',
  DmPolicy.following: 'following',
  DmPolicy.nobody: 'nobody',
  DmPolicy.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$MentionPolicyEnumMap = {
  MentionPolicy.everyone: 'everyone',
  MentionPolicy.following: 'following',
  MentionPolicy.nobody: 'nobody',
  MentionPolicy.unknownDefaultOpenApi: 'unknown_default_open_api',
};
