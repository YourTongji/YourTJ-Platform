//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'dm_report.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmReport {
  /// Returns a new [DmReport] instance.
  DmReport({
    required this.id,

    required this.messageId,

    required this.conversationId,

    required this.reporterId,

    this.reporterHandle,

    this.reporterDisplayName,

    required this.senderId,

    this.senderHandle,

    this.senderDisplayName,

    this.messageExcerpt,

    required this.reason,

    this.note,

    required this.status,

    this.handledBy,

    this.handledAt,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'messageId', required: true, includeIfNull: false)
  final String messageId;

  @JsonKey(name: r'conversationId', required: true, includeIfNull: false)
  final String conversationId;

  @JsonKey(name: r'reporterId', required: true, includeIfNull: false)
  final String reporterId;

  @JsonKey(name: r'reporterHandle', required: false, includeIfNull: false)
  final String? reporterHandle;

  @JsonKey(name: r'reporterDisplayName', required: false, includeIfNull: false)
  final String? reporterDisplayName;

  @JsonKey(name: r'senderId', required: true, includeIfNull: false)
  final String senderId;

  @JsonKey(name: r'senderHandle', required: false, includeIfNull: false)
  final String? senderHandle;

  @JsonKey(name: r'senderDisplayName', required: false, includeIfNull: false)
  final String? senderDisplayName;

  @JsonKey(name: r'messageExcerpt', required: false, includeIfNull: false)
  final String? messageExcerpt;

  @JsonKey(
    name: r'reason',
    required: true,
    includeIfNull: false,
    unknownEnumValue: DmReportReasonEnum.unknownDefaultOpenApi,
  )
  final DmReportReasonEnum reason;

  @JsonKey(name: r'note', required: false, includeIfNull: false)
  final String? note;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: DmReportStatusEnum.unknownDefaultOpenApi,
  )
  final DmReportStatusEnum status;

  @JsonKey(name: r'handledBy', required: false, includeIfNull: false)
  final String? handledBy;

  @JsonKey(name: r'handledAt', required: false, includeIfNull: false)
  final int? handledAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmReport &&
          other.id == id &&
          other.messageId == messageId &&
          other.conversationId == conversationId &&
          other.reporterId == reporterId &&
          other.reporterHandle == reporterHandle &&
          other.reporterDisplayName == reporterDisplayName &&
          other.senderId == senderId &&
          other.senderHandle == senderHandle &&
          other.senderDisplayName == senderDisplayName &&
          other.messageExcerpt == messageExcerpt &&
          other.reason == reason &&
          other.note == note &&
          other.status == status &&
          other.handledBy == handledBy &&
          other.handledAt == handledAt &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      messageId.hashCode +
      conversationId.hashCode +
      reporterId.hashCode +
      reporterHandle.hashCode +
      (reporterDisplayName == null ? 0 : reporterDisplayName.hashCode) +
      senderId.hashCode +
      senderHandle.hashCode +
      (senderDisplayName == null ? 0 : senderDisplayName.hashCode) +
      messageExcerpt.hashCode +
      reason.hashCode +
      (note == null ? 0 : note.hashCode) +
      status.hashCode +
      (handledBy == null ? 0 : handledBy.hashCode) +
      (handledAt == null ? 0 : handledAt.hashCode) +
      createdAt.hashCode;

  factory DmReport.fromJson(Map<String, dynamic> json) =>
      _$DmReportFromJson(json);

  Map<String, dynamic> toJson() => _$DmReportToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum DmReportReasonEnum {
  @JsonValue(r'spam')
  spam(r'spam'),
  @JsonValue(r'abuse')
  abuse(r'abuse'),
  @JsonValue(r'harassment')
  harassment(r'harassment'),
  @JsonValue(r'fraud')
  fraud(r'fraud'),
  @JsonValue(r'illegal')
  illegal(r'illegal'),
  @JsonValue(r'other')
  other(r'other'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DmReportReasonEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum DmReportStatusEnum {
  @JsonValue(r'open')
  open(r'open'),
  @JsonValue(r'upheld')
  upheld(r'upheld'),
  @JsonValue(r'rejected')
  rejected(r'rejected'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DmReportStatusEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
