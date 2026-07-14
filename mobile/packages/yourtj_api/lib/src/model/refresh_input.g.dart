// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'refresh_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

RefreshInput _$RefreshInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('RefreshInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['refreshToken']);
      final val = RefreshInput(
        refreshToken: $checkedConvert('refreshToken', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$RefreshInputToJson(RefreshInput instance) =>
    <String, dynamic>{'refreshToken': instance.refreshToken};
