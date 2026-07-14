// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'feature_thread_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

FeatureThreadInput _$FeatureThreadInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('FeatureThreadInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['featured', 'reason']);
      final val = FeatureThreadInput(
        featured: $checkedConvert('featured', (v) => v as bool),
        reason: $checkedConvert('reason', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$FeatureThreadInputToJson(FeatureThreadInput instance) =>
    <String, dynamic>{'featured': instance.featured, 'reason': instance.reason};
