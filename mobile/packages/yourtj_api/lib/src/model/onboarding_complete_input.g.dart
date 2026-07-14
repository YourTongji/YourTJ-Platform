// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'onboarding_complete_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

OnboardingCompleteInput _$OnboardingCompleteInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('OnboardingCompleteInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'handle',
      'displayName',
      'bio',
      'profileVisibility',
      'activityVisibility',
      'discoverable',
      'acceptedTermsVersion',
    ],
  );
  final val = OnboardingCompleteInput(
    handle: $checkedConvert('handle', (v) => v as String),
    displayName: $checkedConvert('displayName', (v) => v as String?),
    bio: $checkedConvert('bio', (v) => v as String?),
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
      (v) => $enumDecode(
        _$ActivityVisibilityEnumMap,
        v,
        unknownValue: ActivityVisibility.unknownDefaultOpenApi,
      ),
    ),
    discoverable: $checkedConvert('discoverable', (v) => v as bool),
    acceptedTermsVersion: $checkedConvert(
      'acceptedTermsVersion',
      (v) => v as String,
    ),
  );
  return val;
});

Map<String, dynamic> _$OnboardingCompleteInputToJson(
  OnboardingCompleteInput instance,
) => <String, dynamic>{
  'handle': instance.handle,
  'displayName': instance.displayName,
  'bio': instance.bio,
  'profileVisibility': _$ProfileVisibilityEnumMap[instance.profileVisibility]!,
  'activityVisibility':
      _$ActivityVisibilityEnumMap[instance.activityVisibility]!,
  'discoverable': instance.discoverable,
  'acceptedTermsVersion': instance.acceptedTermsVersion,
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
