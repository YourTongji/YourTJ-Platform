// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'my_profile.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MyProfile _$MyProfileFromJson(Map<String, dynamic> json) =>
    $checkedCreate('MyProfile', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'accountId',
          'displayName',
          'school',
          'bio',
          'website',
          'avatarAssetId',
          'bannerAssetId',
        ],
      );
      final val = MyProfile(
        accountId: $checkedConvert('accountId', (v) => v as String),
        displayName: $checkedConvert('displayName', (v) => v as String?),
        school: $checkedConvert('school', (v) => v as String),
        bio: $checkedConvert('bio', (v) => v as String?),
        website: $checkedConvert('website', (v) => v as String?),
        avatarAssetId: $checkedConvert('avatarAssetId', (v) => v as String?),
        bannerAssetId: $checkedConvert('bannerAssetId', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$MyProfileToJson(MyProfile instance) => <String, dynamic>{
  'accountId': instance.accountId,
  'displayName': instance.displayName,
  'school': instance.school,
  'bio': instance.bio,
  'website': instance.website,
  'avatarAssetId': instance.avatarAssetId,
  'bannerAssetId': instance.bannerAssetId,
};
