// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'activity_day.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ActivityDay _$ActivityDayFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ActivityDay', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'date',
          'threads',
          'comments',
          'likes',
          'checkIns',
          'score',
        ],
      );
      final val = ActivityDay(
        date: $checkedConvert('date', (v) => DateTime.parse(v as String)),
        threads: $checkedConvert('threads', (v) => (v as num).toInt()),
        comments: $checkedConvert('comments', (v) => (v as num).toInt()),
        likes: $checkedConvert('likes', (v) => (v as num).toInt()),
        checkIns: $checkedConvert('checkIns', (v) => (v as num).toInt()),
        score: $checkedConvert('score', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$ActivityDayToJson(ActivityDay instance) =>
    <String, dynamic>{
      'date': instance.date.toIso8601String(),
      'threads': instance.threads,
      'comments': instance.comments,
      'likes': instance.likes,
      'checkIns': instance.checkIns,
      'score': instance.score,
    };
