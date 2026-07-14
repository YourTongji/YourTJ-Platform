// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'governance_notice_read_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

GovernanceNoticeReadInput _$GovernanceNoticeReadInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('GovernanceNoticeReadInput', json, ($checkedConvert) {
  final val = GovernanceNoticeReadInput(
    ids: $checkedConvert(
      'ids',
      (v) => (v as List<dynamic>?)?.map((e) => e as String).toList(),
    ),
    all: $checkedConvert('all', (v) => v as bool?),
  );
  return val;
});

Map<String, dynamic> _$GovernanceNoticeReadInputToJson(
  GovernanceNoticeReadInput instance,
) => <String, dynamic>{'ids': ?instance.ids, 'all': ?instance.all};
