// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_message_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmMessageInput _$DmMessageInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DmMessageInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['body']);
      final val = DmMessageInput(
        body: $checkedConvert('body', (v) => v as String),
        clientMessageId: $checkedConvert(
          'clientMessageId',
          (v) => v as String?,
        ),
      );
      return val;
    });

Map<String, dynamic> _$DmMessageInputToJson(DmMessageInput instance) =>
    <String, dynamic>{
      'body': instance.body,
      'clientMessageId': ?instance.clientMessageId,
    };
