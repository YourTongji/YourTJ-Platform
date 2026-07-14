// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'appeal_history.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AppealHistory _$AppealHistoryFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AppealHistory', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['id', 'toStatus', 'reason', 'createdAt'],
      );
      final val = AppealHistory(
        id: $checkedConvert('id', (v) => v as String),
        fromStatus: $checkedConvert(
          'fromStatus',
          (v) => $enumDecodeNullable(
            _$AppealStatusEnumMap,
            v,
            unknownValue: AppealStatus.unknownDefaultOpenApi,
          ),
        ),
        toStatus: $checkedConvert(
          'toStatus',
          (v) => $enumDecode(
            _$AppealStatusEnumMap,
            v,
            unknownValue: AppealStatus.unknownDefaultOpenApi,
          ),
        ),
        reason: $checkedConvert('reason', (v) => v as String),
        metadata: $checkedConvert('metadata', (v) => v),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$AppealHistoryToJson(AppealHistory instance) =>
    <String, dynamic>{
      'id': instance.id,
      'fromStatus': ?_$AppealStatusEnumMap[instance.fromStatus],
      'toStatus': _$AppealStatusEnumMap[instance.toStatus]!,
      'reason': instance.reason,
      'metadata': ?instance.metadata,
      'createdAt': instance.createdAt,
    };

const _$AppealStatusEnumMap = {
  AppealStatus.submitted: 'submitted',
  AppealStatus.inReview: 'in_review',
  AppealStatus.upheld: 'upheld',
  AppealStatus.overturned: 'overturned',
  AppealStatus.amended: 'amended',
  AppealStatus.withdrawn: 'withdrawn',
  AppealStatus.unknownDefaultOpenApi: 'unknown_default_open_api',
};
