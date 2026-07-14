// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_read_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmReadInput _$DmReadInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DmReadInput', json, ($checkedConvert) {
      final val = DmReadInput(
        lastReadMessageId: $checkedConvert(
          'lastReadMessageId',
          (v) => v as String?,
        ),
      );
      return val;
    });

Map<String, dynamic> _$DmReadInputToJson(DmReadInput instance) =>
    <String, dynamic>{'lastReadMessageId': ?instance.lastReadMessageId};
