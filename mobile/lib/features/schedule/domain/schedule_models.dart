import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

@immutable
class ScheduleNamespace {
  const ScheduleNamespace({required this.environment, required this.principal});

  final String environment;
  final String principal;

  String storageKey(String calendarId) {
    String encode(String value) =>
        base64Url.encode(utf8.encode(value)).replaceAll('=', '');
    return 'yourtj.schedule.v2.${encode(environment)}.'
        '${encode(principal)}.${encode(calendarId)}';
  }
}

@immutable
class ScheduledCourse {
  const ScheduledCourse({
    required this.course,
    required this.timeslots,
    required this.colorIndex,
  });

  final SelectionCourse course;
  final List<TimeSlot> timeslots;
  final int colorIndex;

  bool get hasUnknownWeeks => timeslots.any(
    (TimeSlot timeslot) => timeslot.weeks?.trim().isNotEmpty != true,
  );
}

enum ScheduleConflictKind { confirmed, possible }

@immutable
class ScheduleConflict {
  const ScheduleConflict({
    required this.kind,
    required this.withCourse,
    required this.existingSlot,
    required this.candidateSlot,
  });

  final ScheduleConflictKind kind;
  final ScheduledCourse withCourse;
  final TimeSlot existingSlot;
  final TimeSlot candidateSlot;
}

enum ScheduleAddStatus { added, duplicate, conflict }

@immutable
class ScheduleAddResult {
  const ScheduleAddResult._({
    required this.status,
    this.conflict,
    this.pendingCourse,
    this.pendingTimeslots,
  });

  const ScheduleAddResult.added() : this._(status: ScheduleAddStatus.added);

  const ScheduleAddResult.duplicate()
    : this._(status: ScheduleAddStatus.duplicate);

  const ScheduleAddResult.conflict({
    required ScheduleConflict conflict,
    required SelectionCourse pendingCourse,
    required List<TimeSlot> pendingTimeslots,
  }) : this._(
         status: ScheduleAddStatus.conflict,
         conflict: conflict,
         pendingCourse: pendingCourse,
         pendingTimeslots: pendingTimeslots,
       );

  final ScheduleAddStatus status;
  final ScheduleConflict? conflict;
  final SelectionCourse? pendingCourse;
  final List<TimeSlot>? pendingTimeslots;
}

ScheduleConflict? findScheduleConflict({
  required List<ScheduledCourse> existing,
  required List<TimeSlot> candidate,
}) {
  ScheduleConflict? possible;
  for (final ScheduledCourse scheduled in existing) {
    for (final TimeSlot existingSlot in scheduled.timeslots) {
      for (final TimeSlot candidateSlot in candidate) {
        if (!_overlapsBySlot(existingSlot, candidateSlot)) {
          continue;
        }
        final ScheduleConflictKind? kind = _weekConflictKind(
          existingSlot.weeks,
          candidateSlot.weeks,
        );
        if (kind == null) {
          continue;
        }
        final ScheduleConflict conflict = ScheduleConflict(
          kind: kind,
          withCourse: scheduled,
          existingSlot: existingSlot,
          candidateSlot: candidateSlot,
        );
        if (kind == ScheduleConflictKind.confirmed) {
          return conflict;
        }
        possible ??= conflict;
      }
    }
  }
  return possible;
}

bool _overlapsBySlot(TimeSlot left, TimeSlot right) {
  if (left.weekday != right.weekday) {
    return false;
  }
  return left.startSlot <= right.endSlot && right.startSlot <= left.endSlot;
}

ScheduleConflictKind? _weekConflictKind(String? left, String? right) {
  final String leftValue = left?.trim() ?? '';
  final String rightValue = right?.trim() ?? '';
  if (leftValue.isEmpty || rightValue.isEmpty) {
    return ScheduleConflictKind.possible;
  }
  final Set<int>? leftWeeks = parseCourseWeeks(leftValue);
  final Set<int>? rightWeeks = parseCourseWeeks(rightValue);
  if (leftWeeks == null || rightWeeks == null) {
    return leftValue == rightValue
        ? ScheduleConflictKind.confirmed
        : ScheduleConflictKind.possible;
  }
  return leftWeeks.any(rightWeeks.contains)
      ? ScheduleConflictKind.confirmed
      : null;
}

Set<int>? parseCourseWeeks(String value) {
  String normalized = value
      .trim()
      .replaceAll(RegExp(r'\s+'), '')
      .replaceAll('周', '')
      .replaceAll('至', '-')
      .replaceAll('—', '-')
      .replaceAll('–', '-')
      .replaceAll('~', '-')
      .replaceAll('，', ',')
      .replaceAll('、', ',')
      .replaceAll(';', ',')
      .replaceAll('；', ',');
  if (normalized.isEmpty || normalized.length > 128) {
    return null;
  }
  String? globalParity;
  if (normalized.endsWith('单') || normalized.endsWith('双')) {
    globalParity = normalized.substring(normalized.length - 1);
    normalized = normalized.substring(0, normalized.length - 1);
  }
  final Set<int> weeks = <int>{};
  final List<String> segments = normalized.split(',');
  if (segments.isEmpty || segments.length > 60) {
    return null;
  }
  for (String segment in segments) {
    if (segment.isEmpty) {
      return null;
    }
    String? parity = globalParity;
    if (segment.endsWith('单') || segment.endsWith('双')) {
      parity = segment.substring(segment.length - 1);
      segment = segment.substring(0, segment.length - 1);
    }
    final RegExpMatch? match = RegExp(
      r'^(\d{1,2})(?:-(\d{1,2}))?$',
    ).firstMatch(segment);
    if (match == null) {
      return null;
    }
    final int? start = int.tryParse(match.group(1)!);
    final int? end = int.tryParse(match.group(2) ?? match.group(1)!);
    if (start == null || end == null || start < 1 || end > 60 || start > end) {
      return null;
    }
    for (int week = start; week <= end; week += 1) {
      if (parity == '单' && week.isEven) {
        continue;
      }
      if (parity == '双' && week.isOdd) {
        continue;
      }
      weeks.add(week);
    }
  }
  return weeks.isEmpty ? null : weeks;
}
