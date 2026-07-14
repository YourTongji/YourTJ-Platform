// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trust_level_policy.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TrustLevelPolicy _$TrustLevelPolicyFromJson(Map<String, dynamic> json) =>
    $checkedCreate('TrustLevelPolicy', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'version',
          'scorePolicyVersion',
          'thresholdLevel2',
          'thresholdLevel3',
          'thresholdLevel4',
          'thresholdLevel5',
          'thresholdLevel6',
          'likeDailyCap',
          'demotionCooldownDays',
          'reason',
          'changedBy',
          'createdAt',
        ],
      );
      final val = TrustLevelPolicy(
        version: $checkedConvert('version', (v) => (v as num).toInt()),
        scorePolicyVersion: $checkedConvert(
          'scorePolicyVersion',
          (v) => (v as num).toInt(),
        ),
        thresholdLevel2: $checkedConvert(
          'thresholdLevel2',
          (v) => (v as num).toInt(),
        ),
        thresholdLevel3: $checkedConvert(
          'thresholdLevel3',
          (v) => (v as num).toInt(),
        ),
        thresholdLevel4: $checkedConvert(
          'thresholdLevel4',
          (v) => (v as num).toInt(),
        ),
        thresholdLevel5: $checkedConvert(
          'thresholdLevel5',
          (v) => (v as num).toInt(),
        ),
        thresholdLevel6: $checkedConvert(
          'thresholdLevel6',
          (v) => (v as num).toInt(),
        ),
        likeDailyCap: $checkedConvert(
          'likeDailyCap',
          (v) => (v as num).toInt(),
        ),
        demotionCooldownDays: $checkedConvert(
          'demotionCooldownDays',
          (v) => (v as num).toInt(),
        ),
        reason: $checkedConvert('reason', (v) => v as String),
        changedBy: $checkedConvert('changedBy', (v) => v as String),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$TrustLevelPolicyToJson(TrustLevelPolicy instance) =>
    <String, dynamic>{
      'version': instance.version,
      'scorePolicyVersion': instance.scorePolicyVersion,
      'thresholdLevel2': instance.thresholdLevel2,
      'thresholdLevel3': instance.thresholdLevel3,
      'thresholdLevel4': instance.thresholdLevel4,
      'thresholdLevel5': instance.thresholdLevel5,
      'thresholdLevel6': instance.thresholdLevel6,
      'likeDailyCap': instance.likeDailyCap,
      'demotionCooldownDays': instance.demotionCooldownDays,
      'reason': instance.reason,
      'changedBy': instance.changedBy,
      'createdAt': instance.createdAt,
    };
