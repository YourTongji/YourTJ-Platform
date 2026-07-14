// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'poll.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Poll _$PollFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Poll', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'question',
          'multiSelect',
          'closesAt',
          'options',
          'myVotes',
        ],
      );
      final val = Poll(
        id: $checkedConvert('id', (v) => v as String),
        question: $checkedConvert('question', (v) => v as String),
        multiSelect: $checkedConvert('multiSelect', (v) => v as bool),
        closesAt: $checkedConvert('closesAt', (v) => (v as num?)?.toInt()),
        options: $checkedConvert(
          'options',
          (v) => (v as List<dynamic>)
              .map((e) => PollOption.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        myVotes: $checkedConvert(
          'myVotes',
          (v) => (v as List<dynamic>).map((e) => e as String).toList(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$PollToJson(Poll instance) => <String, dynamic>{
  'id': instance.id,
  'question': instance.question,
  'multiSelect': instance.multiSelect,
  'closesAt': instance.closesAt,
  'options': instance.options.map((e) => e.toJson()).toList(),
  'myVotes': instance.myVotes,
};
