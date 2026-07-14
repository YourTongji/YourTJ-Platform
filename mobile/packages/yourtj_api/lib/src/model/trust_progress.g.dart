// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trust_progress.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TrustProgress _$TrustProgressFromJson(Map<String, dynamic> json) =>
    $checkedCreate('TrustProgress', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'trustLevel',
          'teaName',
          'qualifyingScore',
          'nextLevel',
          'nextThreshold',
          'remainingScore',
          'progressPercent',
          'policyVersion',
          'isMaxLevel',
          'overrideActive',
          'promotionBlockedUntil',
          'promotionRequiresNewActivity',
        ],
      );
      final val = TrustProgress(
        trustLevel: $checkedConvert('trustLevel', (v) => (v as num).toInt()),
        teaName: $checkedConvert('teaName', (v) => v as String),
        qualifyingScore: $checkedConvert(
          'qualifyingScore',
          (v) => (v as num).toInt(),
        ),
        nextLevel: $checkedConvert('nextLevel', (v) => (v as num?)?.toInt()),
        nextThreshold: $checkedConvert(
          'nextThreshold',
          (v) => (v as num?)?.toInt(),
        ),
        remainingScore: $checkedConvert(
          'remainingScore',
          (v) => (v as num?)?.toInt(),
        ),
        progressPercent: $checkedConvert(
          'progressPercent',
          (v) => (v as num).toInt(),
        ),
        policyVersion: $checkedConvert(
          'policyVersion',
          (v) => (v as num).toInt(),
        ),
        isMaxLevel: $checkedConvert('isMaxLevel', (v) => v as bool),
        overrideActive: $checkedConvert('overrideActive', (v) => v as bool),
        promotionBlockedUntil: $checkedConvert(
          'promotionBlockedUntil',
          (v) => (v as num?)?.toInt(),
        ),
        promotionRequiresNewActivity: $checkedConvert(
          'promotionRequiresNewActivity',
          (v) => v as bool,
        ),
      );
      return val;
    });

Map<String, dynamic> _$TrustProgressToJson(TrustProgress instance) =>
    <String, dynamic>{
      'trustLevel': instance.trustLevel,
      'teaName': instance.teaName,
      'qualifyingScore': instance.qualifyingScore,
      'nextLevel': instance.nextLevel,
      'nextThreshold': instance.nextThreshold,
      'remainingScore': instance.remainingScore,
      'progressPercent': instance.progressPercent,
      'policyVersion': instance.policyVersion,
      'isMaxLevel': instance.isMaxLevel,
      'overrideActive': instance.overrideActive,
      'promotionBlockedUntil': instance.promotionBlockedUntil,
      'promotionRequiresNewActivity': instance.promotionRequiresNewActivity,
    };
