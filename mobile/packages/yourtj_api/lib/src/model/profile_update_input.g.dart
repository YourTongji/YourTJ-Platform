// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'profile_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ProfileUpdateInput _$ProfileUpdateInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ProfileUpdateInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['displayName', 'bio', 'website']);
      final val = ProfileUpdateInput(
        displayName: $checkedConvert('displayName', (v) => v as String?),
        school: $checkedConvert('school', (v) => v as String?),
        bio: $checkedConvert('bio', (v) => v as String?),
        website: $checkedConvert('website', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$ProfileUpdateInputToJson(ProfileUpdateInput instance) =>
    <String, dynamic>{
      'displayName': instance.displayName,
      'school': ?instance.school,
      'bio': instance.bio,
      'website': instance.website,
    };
