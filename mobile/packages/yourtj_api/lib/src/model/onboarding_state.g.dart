// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'onboarding_state.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

OnboardingState _$OnboardingStateFromJson(Map<String, dynamic> json) =>
    $checkedCreate('OnboardingState', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'required',
          'currentTermsVersion',
          'acceptedTermsVersion',
          'handle',
          'displayName',
          'bio',
          'profileVisibility',
          'activityVisibility',
          'discoverable',
          'completedAt',
        ],
      );
      final val = OnboardingState(
        required_: $checkedConvert('required', (v) => v as bool),
        currentTermsVersion: $checkedConvert(
          'currentTermsVersion',
          (v) => v as String,
        ),
        acceptedTermsVersion: $checkedConvert(
          'acceptedTermsVersion',
          (v) => v as String?,
        ),
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
        completedAt: $checkedConvert(
          'completedAt',
          (v) => (v as num?)?.toInt(),
        ),
      );
      return val;
    }, fieldKeyMap: const {'required_': 'required'});

Map<String, dynamic> _$OnboardingStateToJson(
  OnboardingState instance,
) => <String, dynamic>{
  'required': instance.required_,
  'currentTermsVersion': instance.currentTermsVersion,
  'acceptedTermsVersion': instance.acceptedTermsVersion,
  'handle': instance.handle,
  'displayName': instance.displayName,
  'bio': instance.bio,
  'profileVisibility': _$ProfileVisibilityEnumMap[instance.profileVisibility]!,
  'activityVisibility':
      _$ActivityVisibilityEnumMap[instance.activityVisibility]!,
  'discoverable': instance.discoverable,
  'completedAt': instance.completedAt,
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
