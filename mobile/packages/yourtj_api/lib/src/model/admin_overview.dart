//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'admin_overview.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AdminOverview {
  /// Returns a new [AdminOverview] instance.
  AdminOverview({
    required this.totalUsers,

    required this.activeUsers,

    required this.suspendedUsers,

    this.moderators,

    this.administrators,

    required this.pendingReviewReports,

    required this.pendingForumFlags,

    required this.pendingDmReports,

    required this.pendingMediaUploads,

    required this.threadsToday,

    required this.commentsToday,

    required this.likesToday,
  });

  // minimum: 0
  @JsonKey(name: r'totalUsers', required: true, includeIfNull: false)
  final int totalUsers;

  // minimum: 0
  @JsonKey(name: r'activeUsers', required: true, includeIfNull: false)
  final int activeUsers;

  // minimum: 0
  @JsonKey(name: r'suspendedUsers', required: true, includeIfNull: false)
  final int suspendedUsers;

  // minimum: 0
  @JsonKey(name: r'moderators', required: false, includeIfNull: false)
  final int? moderators;

  // minimum: 0
  @JsonKey(name: r'administrators', required: false, includeIfNull: false)
  final int? administrators;

  // minimum: 0
  @JsonKey(name: r'pendingReviewReports', required: true, includeIfNull: false)
  final int pendingReviewReports;

  // minimum: 0
  @JsonKey(name: r'pendingForumFlags', required: true, includeIfNull: false)
  final int pendingForumFlags;

  // minimum: 0
  @JsonKey(name: r'pendingDmReports', required: true, includeIfNull: false)
  final int pendingDmReports;

  // minimum: 0
  @JsonKey(name: r'pendingMediaUploads', required: true, includeIfNull: false)
  final int pendingMediaUploads;

  // minimum: 0
  @JsonKey(name: r'threadsToday', required: true, includeIfNull: false)
  final int threadsToday;

  // minimum: 0
  @JsonKey(name: r'commentsToday', required: true, includeIfNull: false)
  final int commentsToday;

  // minimum: 0
  @JsonKey(name: r'likesToday', required: true, includeIfNull: false)
  final int likesToday;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AdminOverview &&
          other.totalUsers == totalUsers &&
          other.activeUsers == activeUsers &&
          other.suspendedUsers == suspendedUsers &&
          other.moderators == moderators &&
          other.administrators == administrators &&
          other.pendingReviewReports == pendingReviewReports &&
          other.pendingForumFlags == pendingForumFlags &&
          other.pendingDmReports == pendingDmReports &&
          other.pendingMediaUploads == pendingMediaUploads &&
          other.threadsToday == threadsToday &&
          other.commentsToday == commentsToday &&
          other.likesToday == likesToday;

  @override
  int get hashCode =>
      totalUsers.hashCode +
      activeUsers.hashCode +
      suspendedUsers.hashCode +
      moderators.hashCode +
      administrators.hashCode +
      pendingReviewReports.hashCode +
      pendingForumFlags.hashCode +
      pendingDmReports.hashCode +
      pendingMediaUploads.hashCode +
      threadsToday.hashCode +
      commentsToday.hashCode +
      likesToday.hashCode;

  factory AdminOverview.fromJson(Map<String, dynamic> json) =>
      _$AdminOverviewFromJson(json);

  Map<String, dynamic> toJson() => _$AdminOverviewToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
