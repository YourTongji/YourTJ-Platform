import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

@immutable
class CourseListEntry {
  const CourseListEntry({
    required this.id,
    required this.code,
    required this.name,
    required this.reviewCount,
    this.credit,
    this.department,
    this.teacherName,
    this.reviewAverage,
  });

  factory CourseListEntry.fromCourse(Course course) {
    return CourseListEntry(
      id: course.id ?? '',
      code: course.code ?? '',
      name: course.name?.trim().isNotEmpty == true
          ? course.name!.trim()
          : '未命名课程',
      credit: course.credit,
      department: course.department,
      teacherName: course.teacherName,
      reviewCount: course.reviewCount ?? 0,
      reviewAverage: course.reviewAvg,
    );
  }

  factory CourseListEntry.fromSearchHit(CourseSearchHit course) {
    return CourseListEntry(
      id: course.id,
      code: course.code,
      name: course.name,
      credit: course.credit,
      department: course.department,
      teacherName: course.teacherName,
      reviewCount: course.reviewCount,
      reviewAverage: course.reviewAvg,
    );
  }

  final String id;
  final String code;
  final String name;
  final num? credit;
  final String? department;
  final String? teacherName;
  final int reviewCount;
  final num? reviewAverage;
}

@immutable
class CoursePageSlice {
  const CoursePageSlice({
    required this.items,
    required this.nextCursor,
    required this.hasMore,
  });

  final List<CourseListEntry> items;
  final String? nextCursor;
  final bool hasMore;
}
