// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_overview.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminOverview _$AdminOverviewFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminOverview', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'totalUsers',
      'activeUsers',
      'suspendedUsers',
      'pendingReviewReports',
      'pendingForumFlags',
      'pendingDmReports',
      'pendingMediaUploads',
      'threadsToday',
      'commentsToday',
      'likesToday',
    ],
  );
  final val = AdminOverview(
    totalUsers: $checkedConvert('totalUsers', (v) => (v as num).toInt()),
    activeUsers: $checkedConvert('activeUsers', (v) => (v as num).toInt()),
    suspendedUsers: $checkedConvert(
      'suspendedUsers',
      (v) => (v as num).toInt(),
    ),
    moderators: $checkedConvert('moderators', (v) => (v as num?)?.toInt()),
    administrators: $checkedConvert(
      'administrators',
      (v) => (v as num?)?.toInt(),
    ),
    pendingReviewReports: $checkedConvert(
      'pendingReviewReports',
      (v) => (v as num).toInt(),
    ),
    pendingForumFlags: $checkedConvert(
      'pendingForumFlags',
      (v) => (v as num).toInt(),
    ),
    pendingDmReports: $checkedConvert(
      'pendingDmReports',
      (v) => (v as num).toInt(),
    ),
    pendingMediaUploads: $checkedConvert(
      'pendingMediaUploads',
      (v) => (v as num).toInt(),
    ),
    threadsToday: $checkedConvert('threadsToday', (v) => (v as num).toInt()),
    commentsToday: $checkedConvert('commentsToday', (v) => (v as num).toInt()),
    likesToday: $checkedConvert('likesToday', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$AdminOverviewToJson(AdminOverview instance) =>
    <String, dynamic>{
      'totalUsers': instance.totalUsers,
      'activeUsers': instance.activeUsers,
      'suspendedUsers': instance.suspendedUsers,
      'moderators': ?instance.moderators,
      'administrators': ?instance.administrators,
      'pendingReviewReports': instance.pendingReviewReports,
      'pendingForumFlags': instance.pendingForumFlags,
      'pendingDmReports': instance.pendingDmReports,
      'pendingMediaUploads': instance.pendingMediaUploads,
      'threadsToday': instance.threadsToday,
      'commentsToday': instance.commentsToday,
      'likesToday': instance.likesToday,
    };
