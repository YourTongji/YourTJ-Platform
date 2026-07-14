// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_report.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmReport _$DmReportFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DmReport', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'messageId',
          'conversationId',
          'reporterId',
          'senderId',
          'reason',
          'status',
          'createdAt',
        ],
      );
      final val = DmReport(
        id: $checkedConvert('id', (v) => v as String),
        messageId: $checkedConvert('messageId', (v) => v as String),
        conversationId: $checkedConvert('conversationId', (v) => v as String),
        reporterId: $checkedConvert('reporterId', (v) => v as String),
        reporterHandle: $checkedConvert('reporterHandle', (v) => v as String?),
        reporterDisplayName: $checkedConvert(
          'reporterDisplayName',
          (v) => v as String?,
        ),
        senderId: $checkedConvert('senderId', (v) => v as String),
        senderHandle: $checkedConvert('senderHandle', (v) => v as String?),
        senderDisplayName: $checkedConvert(
          'senderDisplayName',
          (v) => v as String?,
        ),
        messageExcerpt: $checkedConvert('messageExcerpt', (v) => v as String?),
        reason: $checkedConvert(
          'reason',
          (v) => $enumDecode(
            _$DmReportReasonEnumEnumMap,
            v,
            unknownValue: DmReportReasonEnum.unknownDefaultOpenApi,
          ),
        ),
        note: $checkedConvert('note', (v) => v as String?),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$DmReportStatusEnumEnumMap,
            v,
            unknownValue: DmReportStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        handledBy: $checkedConvert('handledBy', (v) => v as String?),
        handledAt: $checkedConvert('handledAt', (v) => (v as num?)?.toInt()),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$DmReportToJson(DmReport instance) => <String, dynamic>{
  'id': instance.id,
  'messageId': instance.messageId,
  'conversationId': instance.conversationId,
  'reporterId': instance.reporterId,
  'reporterHandle': ?instance.reporterHandle,
  'reporterDisplayName': ?instance.reporterDisplayName,
  'senderId': instance.senderId,
  'senderHandle': ?instance.senderHandle,
  'senderDisplayName': ?instance.senderDisplayName,
  'messageExcerpt': ?instance.messageExcerpt,
  'reason': _$DmReportReasonEnumEnumMap[instance.reason]!,
  'note': ?instance.note,
  'status': _$DmReportStatusEnumEnumMap[instance.status]!,
  'handledBy': ?instance.handledBy,
  'handledAt': ?instance.handledAt,
  'createdAt': instance.createdAt,
};

const _$DmReportReasonEnumEnumMap = {
  DmReportReasonEnum.spam: 'spam',
  DmReportReasonEnum.abuse: 'abuse',
  DmReportReasonEnum.harassment: 'harassment',
  DmReportReasonEnum.fraud: 'fraud',
  DmReportReasonEnum.illegal: 'illegal',
  DmReportReasonEnum.other: 'other',
  DmReportReasonEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$DmReportStatusEnumEnumMap = {
  DmReportStatusEnum.open: 'open',
  DmReportStatusEnum.upheld: 'upheld',
  DmReportStatusEnum.rejected: 'rejected',
  DmReportStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
