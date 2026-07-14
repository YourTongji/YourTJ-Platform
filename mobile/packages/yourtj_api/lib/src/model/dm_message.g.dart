// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_message.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmMessage _$DmMessageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DmMessage', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'conversationId',
          'senderId',
          'senderHandle',
          'body',
          'createdAt',
        ],
      );
      final val = DmMessage(
        id: $checkedConvert('id', (v) => v as String),
        conversationId: $checkedConvert('conversationId', (v) => v as String),
        senderId: $checkedConvert('senderId', (v) => v as String),
        senderHandle: $checkedConvert('senderHandle', (v) => v as String),
        senderDisplayName: $checkedConvert(
          'senderDisplayName',
          (v) => v as String?,
        ),
        body: $checkedConvert('body', (v) => v as String),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$DmMessageToJson(DmMessage instance) => <String, dynamic>{
  'id': instance.id,
  'conversationId': instance.conversationId,
  'senderId': instance.senderId,
  'senderHandle': instance.senderHandle,
  'senderDisplayName': ?instance.senderDisplayName,
  'body': instance.body,
  'createdAt': instance.createdAt,
};
