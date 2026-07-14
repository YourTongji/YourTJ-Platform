// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'latest_update.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

LatestUpdate _$LatestUpdateFromJson(Map<String, dynamic> json) =>
    $checkedCreate('LatestUpdate', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['updatedAt']);
      final val = LatestUpdate(
        updatedAt: $checkedConvert(
          'updatedAt',
          (v) => v == null ? null : DateTime.parse(v as String),
        ),
      );
      return val;
    });

Map<String, dynamic> _$LatestUpdateToJson(LatestUpdate instance) =>
    <String, dynamic>{'updatedAt': instance.updatedAt?.toIso8601String()};
