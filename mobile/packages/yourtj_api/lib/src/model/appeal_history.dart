//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/appeal_status.dart';
import 'package:json_annotation/json_annotation.dart';

part 'appeal_history.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AppealHistory {
  /// Returns a new [AppealHistory] instance.
  AppealHistory({
    required this.id,

    this.fromStatus,

    required this.toStatus,

    required this.reason,

    this.metadata,

    required this.createdAt,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(
    name: r'fromStatus',
    required: false,
    includeIfNull: false,
    unknownEnumValue: AppealStatus.unknownDefaultOpenApi,
  )
  final AppealStatus? fromStatus;

  @JsonKey(
    name: r'toStatus',
    required: true,
    includeIfNull: false,
    unknownEnumValue: AppealStatus.unknownDefaultOpenApi,
  )
  final AppealStatus toStatus;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  /// Bounded public decision metadata; currently only a shortened sanction endsAt.
  @JsonKey(name: r'metadata', required: false, includeIfNull: false)
  final Object? metadata;

  @JsonKey(name: r'createdAt', required: true, includeIfNull: false)
  final int createdAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AppealHistory &&
          other.id == id &&
          other.fromStatus == fromStatus &&
          other.toStatus == toStatus &&
          other.reason == reason &&
          other.metadata == metadata &&
          other.createdAt == createdAt;

  @override
  int get hashCode =>
      id.hashCode +
      (fromStatus == null ? 0 : fromStatus.hashCode) +
      toStatus.hashCode +
      reason.hashCode +
      (metadata == null ? 0 : metadata.hashCode) +
      createdAt.hashCode;

  factory AppealHistory.fromJson(Map<String, dynamic> json) =>
      _$AppealHistoryFromJson(json);

  Map<String, dynamic> toJson() => _$AppealHistoryToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
