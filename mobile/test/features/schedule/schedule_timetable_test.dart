import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/schedule/domain/schedule_models.dart';
import 'package:yourtj_mobile/features/schedule/presentation/schedule_page.dart';

void main() {
  testWidgets('uses a touch-friendly day picker on a phone', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ScheduleTimetable(courses: _courses())),
      ),
    );

    expect(find.byType(ChoiceChip), findsNWidgets(7));
    expect(find.text('程序设计'), findsOneWidget);
    expect(find.bySemanticsLabel(RegExp('周一')), findsOneWidget);
    expect(find.bySemanticsLabel(RegExp('程序设计')), findsOneWidget);
    semantics.dispose();
  });

  testWidgets('uses the seven-day table at expanded width', (
    WidgetTester tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ScheduleTimetable(courses: _courses())),
      ),
    );

    expect(find.byType(Table), findsOneWidget);
    expect(find.text('周日'), findsOneWidget);
    expect(find.text('20'), findsOneWidget);
  });
}

List<ScheduledCourse> _courses() {
  return <ScheduledCourse>[
    ScheduledCourse(
      offering: SelectionOffering(
        id: '1',
        offeringId: '1',
        code: 'CS101',
        teachingClassCode: 'CS101-01',
        name: '程序设计',
        credit: 3,
        natureId: null,
        calendarId: '2026-spring',
        campusId: null,
        facultyName: null,
        teachingLanguage: null,
        teacherName: '张老师',
        teacherNames: const <String>['张老师'],
        startWeek: 1,
        endWeek: 16,
        weeksUnknown: false,
        scheduleUnknown: false,
        status: SelectionOfferingStatusEnum.unknown,
        catalogueCourseId: null,
        reviewCount: 0,
        reviewAvg: null,
        reviewScope: SelectionOfferingReviewScopeEnum.none,
      ),
      timeslots: <TimeSlot>[
        TimeSlot(
          offeringId: '1',
          courseId: '1',
          teacherName: '张老师',
          weekday: 1,
          startSlot: 20,
          endSlot: 20,
          weeks: '1-16',
          weekNumbers: const <int>{1, 2, 3, 4},
          weeksUnknown: false,
          location: '教学楼',
          locationUnknown: false,
        ),
      ],
      colorIndex: 0,
    ),
  ];
}
