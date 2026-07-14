//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'governance_notice.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class GovernanceNotice {
  /// Returns a new [GovernanceNotice] instance.
  GovernanceNotice({
    required this.id,

    required this.noticeType,

    required this.subjectKind,

    required this.subjectId,

    required this.summary,

    this.appealId,

    required this.targetUrl,

    required this.read,

    this.readAt,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'noticeType',
    required: true,
    includeIfNull: false,
    unknownEnumValue: GovernanceNoticeNoticeTypeEnum.unknownDefaultOpenApi,
  )
  final GovernanceNoticeNoticeTypeEnum noticeType;

  @JsonKey(
    name: r'subjectKind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: GovernanceNoticeSubjectKindEnum.unknownDefaultOpenApi,
  )
  final GovernanceNoticeSubjectKindEnum subjectKind;

  @JsonKey(name: r'subjectId', required: true, includeIfNull: false)
  final String subjectId;

  @JsonKey(name: r'summary', required: true, includeIfNull: false)
  final String summary;

  @JsonKey(name: r'appealId', required: false, includeIfNull: false)
  final String? appealId;

  @JsonKey(name: r'targetUrl', required: true, includeIfNull: false)
  final String targetUrl;

  @JsonKey(name: r'read', required: true, includeIfNull: false)
  final bool read;

  @JsonKey(name: r'readAt', required: false, includeIfNull: false)
  final int? readAt;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is GovernanceNotice &&
          other.id == id &&
          other.noticeType == noticeType &&
          other.subjectKind == subjectKind &&
          other.subjectId == subjectId &&
          other.summary == summary &&
          other.appealId == appealId &&
          other.targetUrl == targetUrl &&
          other.read == read &&
          other.readAt == readAt &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      noticeType.hashCode +
      subjectKind.hashCode +
      subjectId.hashCode +
      summary.hashCode +
      (appealId == null ? 0 : appealId.hashCode) +
      targetUrl.hashCode +
      read.hashCode +
      (readAt == null ? 0 : readAt.hashCode) +
      createdAt.hashCode;

  factory GovernanceNotice.fromJson(Map<String, dynamic> json) =>
      _$GovernanceNoticeFromJson(json);

  Map<String, dynamic> toJson() => _$GovernanceNoticeToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum GovernanceNoticeNoticeTypeEnum {
  @JsonValue(r'sanction_applied')
  sanctionApplied(r'sanction_applied'),
  @JsonValue(r'content_restricted')
  contentRestricted(r'content_restricted'),
  @JsonValue(r'appeal_submitted')
  appealSubmitted(r'appeal_submitted'),
  @JsonValue(r'appeal_in_review')
  appealInReview(r'appeal_in_review'),
  @JsonValue(r'appeal_upheld')
  appealUpheld(r'appeal_upheld'),
  @JsonValue(r'appeal_overturned')
  appealOverturned(r'appeal_overturned'),
  @JsonValue(r'appeal_amended')
  appealAmended(r'appeal_amended'),
  @JsonValue(r'appeal_withdrawn')
  appealWithdrawn(r'appeal_withdrawn'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const GovernanceNoticeNoticeTypeEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum GovernanceNoticeSubjectKindEnum {
  @JsonValue(r'sanction')
  sanction(r'sanction'),
  @JsonValue(r'forum_thread')
  forumThread(r'forum_thread'),
  @JsonValue(r'forum_comment')
  forumComment(r'forum_comment'),
  @JsonValue(r'review')
  review(r'review'),
  @JsonValue(r'appeal')
  appeal(r'appeal'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const GovernanceNoticeSubjectKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
