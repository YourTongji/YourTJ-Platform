// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'notification_read_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

NotificationReadInput _$NotificationReadInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('NotificationReadInput', json, ($checkedConvert) {
  final val = NotificationReadInput(
    ids: $checkedConvert(
      'ids',
      (v) => (v as List<dynamic>?)?.map((e) => e as String).toList(),
    ),
    all: $checkedConvert('all', (v) => v as bool?),
  );
  return val;
});

Map<String, dynamic> _$NotificationReadInputToJson(
  NotificationReadInput instance,
) => <String, dynamic>{'ids': ?instance.ids, 'all': ?instance.all};
