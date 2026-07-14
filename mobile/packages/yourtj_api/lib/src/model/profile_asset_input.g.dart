// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'profile_asset_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ProfileAssetInput _$ProfileAssetInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ProfileAssetInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['assetId']);
      final val = ProfileAssetInput(
        assetId: $checkedConvert('assetId', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$ProfileAssetInputToJson(ProfileAssetInput instance) =>
    <String, dynamic>{'assetId': instance.assetId};
