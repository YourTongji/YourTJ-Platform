// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'poll_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PollInput _$PollInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PollInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['question', 'options']);
      final val = PollInput(
        question: $checkedConvert('question', (v) => v as String),
        multiSelect: $checkedConvert('multiSelect', (v) => v as bool? ?? false),
        closesAt: $checkedConvert('closesAt', (v) => (v as num?)?.toInt()),
        options: $checkedConvert(
          'options',
          (v) => (v as List<dynamic>).map((e) => e as String).toSet(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$PollInputToJson(PollInput instance) => <String, dynamic>{
  'question': instance.question,
  'multiSelect': ?instance.multiSelect,
  'closesAt': ?instance.closesAt,
  'options': instance.options.toList(),
};
