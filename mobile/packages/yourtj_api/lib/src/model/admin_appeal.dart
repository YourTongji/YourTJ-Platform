//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/appeal_status.dart';
import 'package:yourtj_api/src/model/appeal_history.dart';
import 'package:json_annotation/json_annotation.dart';

part 'admin_appeal.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminAppeal {
  /// Returns a new [AdminAppeal] instance.
  AdminAppeal({
    required this.id,

    required this.governanceEventId,

    required this.originalAction,

    this.originalReason,

    required this.targetKind,

    required this.targetId,

    required this.dispositionKind,

    required this.status,

    required this.submissionReason,

    required this.submittedAt,

    required this.appealableUntil,

    this.reviewStartedAt,

    this.decisionReason,

    this.amendment,

    this.decidedAt,

    required this.version,

    required this.history,

    required this.appellantAccountId,

    this.reviewerAccountId,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'governanceEventId', required: true, includeIfNull: false)
  final String governanceEventId;

  @JsonKey(name: r'originalAction', required: true, includeIfNull: false)
  final String originalAction;

  /// Safe disposition summary for the owner; authorized staff may receive the internal reason.
  @JsonKey(name: r'originalReason', required: false, includeIfNull: false)
  final String? originalReason;

  @JsonKey(
    name: r'targetKind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminAppealTargetKindEnum.unknownDefaultOpenApi,
  )
  final AdminAppealTargetKindEnum targetKind;

  @JsonKey(name: r'targetId', required: true, includeIfNull: false)
  final String targetId;

  @JsonKey(
    name: r'dispositionKind',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AdminAppealDispositionKindEnum.unknownDefaultOpenApi,
  )
  final AdminAppealDispositionKindEnum dispositionKind;

  @JsonKey(
    name: r'status',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AppealStatus.unknownDefaultOpenApi,
  )
  final AppealStatus status;

  @JsonKey(name: r'submissionReason', required: true, includeIfNull: false)
  final String submissionReason;

  @JsonKey(name: r'submittedAt', required: true, includeIfNull: false)
  final int submittedAt;

  @JsonKey(name: r'appealableUntil', required: true, includeIfNull: false)
  final int appealableUntil;

  @JsonKey(name: r'reviewStartedAt', required: false, includeIfNull: false)
  final int? reviewStartedAt;

  @JsonKey(name: r'decisionReason', required: false, includeIfNull: false)
  final String? decisionReason;

  @JsonKey(name: r'amendment', required: false, includeIfNull: false)
  final Object? amendment;

  @JsonKey(name: r'decidedAt', required: false, includeIfNull: false)
  final int? decidedAt;

  // minimum: 1
  @JsonKey(name: r'version', required: true, includeIfNull: false)
  final int version;

  @JsonKey(name: r'history', required: true, includeIfNull: false)
  final List<AppealHistory> history;

  @JsonKey(name: r'appellantAccountId', required: true, includeIfNull: false)
  final String appellantAccountId;

  @JsonKey(name: r'reviewerAccountId', required: false, includeIfNull: false)
  final String? reviewerAccountId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminAppeal &&
          other.id == id &&
          other.governanceEventId == governanceEventId &&
          other.originalAction == originalAction &&
          other.originalReason == originalReason &&
          other.targetKind == targetKind &&
          other.targetId == targetId &&
          other.dispositionKind == dispositionKind &&
          other.status == status &&
          other.submissionReason == submissionReason &&
          other.submittedAt == submittedAt &&
          other.appealableUntil == appealableUntil &&
          other.reviewStartedAt == reviewStartedAt &&
          other.decisionReason == decisionReason &&
          other.amendment == amendment &&
          other.decidedAt == decidedAt &&
          other.version == version &&
          other.history == history &&
          other.appellantAccountId == appellantAccountId &&
          other.reviewerAccountId == reviewerAccountId;

  @override
  int get hashCode =>
      id.hashCode +
      governanceEventId.hashCode +
      originalAction.hashCode +
      (originalReason == null ? 0 : originalReason.hashCode) +
      targetKind.hashCode +
      targetId.hashCode +
      dispositionKind.hashCode +
      status.hashCode +
      submissionReason.hashCode +
      submittedAt.hashCode +
      appealableUntil.hashCode +
      (reviewStartedAt == null ? 0 : reviewStartedAt.hashCode) +
      (decisionReason == null ? 0 : decisionReason.hashCode) +
      (amendment == null ? 0 : amendment.hashCode) +
      (decidedAt == null ? 0 : decidedAt.hashCode) +
      version.hashCode +
      history.hashCode +
      appellantAccountId.hashCode +
      (reviewerAccountId == null ? 0 : reviewerAccountId.hashCode);

  factory AdminAppeal.fromJson(Map<String, dynamic> json) =>
      _$AdminAppealFromJson(json);

  Map<String, dynamic> toJson() => _$AdminAppealToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AdminAppealTargetKindEnum {
  @JsonValue(r'sanction')
  sanction(r'sanction'),
  @JsonValue(r'forum_thread')
  forumThread(r'forum_thread'),
  @JsonValue(r'forum_comment')
  forumComment(r'forum_comment'),
  @JsonValue(r'review')
  review(r'review'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminAppealTargetKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}

enum AdminAppealDispositionKindEnum {
  @JsonValue(r'silence')
  silence(r'silence'),
  @JsonValue(r'suspend')
  suspend(r'suspend'),
  @JsonValue(r'hide')
  hide_(r'hide'),
  @JsonValue(r'delete')
  delete(r'delete'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AdminAppealDispositionKindEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
