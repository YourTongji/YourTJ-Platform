import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../auth/domain/session_state.dart';
import '../data/schedule_local_repository.dart';
import '../data/selection_repository.dart';
import '../domain/schedule_controller.dart';
import '../domain/schedule_models.dart';

class SchedulePage extends ConsumerWidget {
  const SchedulePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final SessionState session =
        ref.watch(sessionStateProvider).value ??
        ref.read(sessionManagerProvider).state;
    final String environment = ref
        .watch(appServicesProvider)
        .environment
        .apiBaseUri
        .toString();
    final String principal = session.account?.id ?? 'anonymous';
    return _ScheduleJourney(
      key: ValueKey<String>('$environment|$principal'),
      namespace: ScheduleNamespace(
        environment: environment,
        principal: principal,
      ),
    );
  }
}

class _ScheduleJourney extends ConsumerStatefulWidget {
  const _ScheduleJourney({required this.namespace, super.key});

  final ScheduleNamespace namespace;

  @override
  ConsumerState<_ScheduleJourney> createState() => _ScheduleJourneyState();
}

class _ScheduleJourneyState extends ConsumerState<_ScheduleJourney> {
  late final ScheduleController _controller;
  final TextEditingController _searchController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _controller = ScheduleController(
      scope: widget.namespace,
      selectionSource: ref.read(selectionRepositoryProvider),
      localSource: ref.read(scheduleLocalRepositoryProvider),
    );
    _controller.initialize();
  }

  @override
  void dispose() {
    _searchController.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: _controller,
      builder: (BuildContext context, Widget? child) {
        if (_controller.isLoading) {
          return const AppLoadingState(
            title: '正在加载选课基础数据',
            description: '正在读取学期、培养方案与本机课表。',
          );
        }
        if (_controller.failure case final ApiFailure failure) {
          return AppErrorState(
            description: failure.message,
            onRetry: _controller.initialize,
          );
        }
        if (_controller.calendars.isEmpty) {
          return const AppEmptyState(
            title: '暂无可用学期',
            description: '选课镜像尚未提供学期数据，请稍后再试。',
          );
        }
        return LayoutBuilder(
          builder: (BuildContext context, BoxConstraints constraints) {
            final bool isExpanded = constraints.maxWidth >= 840;
            return ListView(
              key: const PageStorageKey<String>('selection-schedule'),
              padding: const EdgeInsets.fromLTRB(16, 20, 16, 36),
              children: <Widget>[
                _header(context),
                const SizedBox(height: 14),
                const _ScopeNotice(),
                const SizedBox(height: 16),
                _contextCard(context),
                const SizedBox(height: 16),
                if (isExpanded)
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Expanded(flex: 3, child: _selectionPanel(context)),
                      const SizedBox(width: 18),
                      Expanded(flex: 2, child: _scheduledPanel(context)),
                    ],
                  )
                else ...<Widget>[
                  _selectionPanel(context),
                  const SizedBox(height: 18),
                  _scheduledPanel(context),
                ],
                const SizedBox(height: 22),
                Row(
                  children: <Widget>[
                    Expanded(
                      child: Text(
                        '课表预览',
                        style: Theme.of(context).textTheme.titleLarge,
                      ),
                    ),
                    Text(
                      '${_controller.scheduled.length} 门 · '
                      '${_number(_controller.totalCredits)} 学分',
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                ScheduleTimetable(courses: _controller.scheduled),
              ],
            );
          },
        );
      },
    );
  }

  Widget _header(BuildContext context) {
    final DateTime? updatedAt = _controller.latestUpdate?.updatedAt;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text('选课排课', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 6),
        Text(
          '浏览培养方案课程，完成本机待选、周次提示与冲突检查。',
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          '课程镜像最近同步：${_formatDateTime(updatedAt)}',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }

  Widget _contextCard(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(
                  Icons.calendar_month_outlined,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Text('选课上下文', style: Theme.of(context).textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 14),
            LayoutBuilder(
              builder: (BuildContext context, BoxConstraints constraints) {
                final List<Widget> fields = <Widget>[
                  _dropdown<Calendar>(
                    label: '学期',
                    selectedValue: _controller.calendarId,
                    items: _controller.calendars,
                    value: (Calendar item) => item.id!,
                    name: (Calendar item) => item.name ?? item.id!,
                    onChanged: _controller.selectCalendar,
                  ),
                  _dropdown<String>(
                    label: '年级',
                    selectedValue: _controller.grade,
                    items: _controller.grades,
                    value: (String item) => item,
                    name: (String item) => item,
                    onChanged: _controller.calendarId == null
                        ? null
                        : _controller.selectGrade,
                  ),
                  _dropdown<Major>(
                    label: '专业',
                    selectedValue: _controller.majorId,
                    items: _controller.majors,
                    value: (Major item) => item.id!,
                    name: (Major item) => item.name ?? item.id!,
                    onChanged: _controller.grade == null
                        ? null
                        : _controller.selectMajor,
                  ),
                ];
                if (constraints.maxWidth < 720) {
                  return Column(
                    children:
                        fields
                            .expand(
                              (Widget field) => <Widget>[
                                field,
                                const SizedBox(height: 12),
                              ],
                            )
                            .toList()
                          ..removeLast(),
                  );
                }
                return Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children:
                      fields
                          .expand(
                            (Widget field) => <Widget>[
                              Expanded(child: field),
                              const SizedBox(width: 12),
                            ],
                          )
                          .toList()
                        ..removeLast(),
                );
              },
            ),
            if (_controller.areContextOptionsLoading) ...<Widget>[
              const SizedBox(height: 12),
              const LinearProgressIndicator(),
            ],
            if (_controller.contextFailure
                case final ApiFailure failure) ...<Widget>[
              const SizedBox(height: 12),
              _InlineFailure(message: failure.message),
            ],
            if (_controller.storageFailure
                case final ApiFailure failure) ...<Widget>[
              const SizedBox(height: 12),
              _InlineFailure(message: failure.message),
            ],
          ],
        ),
      ),
    );
  }

  Widget _selectionPanel(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text('查找课程', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 12),
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: SegmentedButton<SelectionBrowseMode>(
                segments: const <ButtonSegment<SelectionBrowseMode>>[
                  ButtonSegment<SelectionBrowseMode>(
                    value: SelectionBrowseMode.major,
                    icon: Icon(Icons.account_tree_outlined),
                    label: Text('培养方案'),
                  ),
                  ButtonSegment<SelectionBrowseMode>(
                    value: SelectionBrowseMode.nature,
                    icon: Icon(Icons.category_outlined),
                    label: Text('按性质'),
                  ),
                  ButtonSegment<SelectionBrowseMode>(
                    value: SelectionBrowseMode.search,
                    icon: Icon(Icons.search_rounded),
                    label: Text('搜索'),
                  ),
                ],
                selected: <SelectionBrowseMode>{_controller.mode},
                onSelectionChanged: (Set<SelectionBrowseMode> selected) {
                  _controller.setMode(selected.first);
                },
              ),
            ),
            const SizedBox(height: 14),
            if (_controller.mode == SelectionBrowseMode.nature)
              _dropdown<CourseNature>(
                label: '课程性质',
                selectedValue: _controller.natureId,
                items: _controller.natures,
                value: (CourseNature item) => item.id!,
                name: (CourseNature item) => item.name ?? item.id!,
                onChanged: _controller.selectNature,
              )
            else if (_controller.mode == SelectionBrowseMode.search)
              Row(
                children: <Widget>[
                  Expanded(
                    child: TextField(
                      controller: _searchController,
                      textInputAction: TextInputAction.search,
                      decoration: const InputDecoration(
                        labelText: '课程名、课号或教师',
                        prefixIcon: Icon(Icons.search_rounded),
                      ),
                      onSubmitted: _controller.submitSearch,
                    ),
                  ),
                  const SizedBox(width: 10),
                  FilledButton(
                    onPressed: () =>
                        _controller.submitSearch(_searchController.text),
                    child: const Text('搜索'),
                  ),
                ],
              ),
            const SizedBox(height: 14),
            _courseResults(context),
          ],
        ),
      ),
    );
  }

  Widget _courseResults(BuildContext context) {
    if (_controller.mode == SelectionBrowseMode.major &&
        _controller.majorId == null) {
      return const _InlineEmpty(title: '先选择年级和专业', message: '选择后会读取该专业培养方案课程。');
    }
    if (_controller.mode == SelectionBrowseMode.nature &&
        _controller.natureId == null) {
      return const _InlineEmpty(title: '选择一个课程性质', message: '例如通识选修、专业选修等。');
    }
    if (_controller.mode == SelectionBrowseMode.search &&
        _controller.query.length < 2 &&
        _controller.coursesFailure == null) {
      return const _InlineEmpty(title: '输入关键词搜索', message: '至少输入 2 个字符。');
    }
    if (_controller.areCoursesLoading) {
      return const Padding(
        padding: EdgeInsets.symmetric(vertical: 36),
        child: Center(child: CircularProgressIndicator()),
      );
    }
    if (_controller.coursesFailure case final ApiFailure failure) {
      return _InlineFailure(message: failure.message);
    }
    if (_controller.courses.isEmpty) {
      return const _InlineEmpty(title: '没有找到课程', message: '更换筛选条件或关键词后再试。');
    }
    return Column(
      children: _controller.courses
          .map((SelectionCourse course) {
            return Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: _SelectionCourseRow(
                course: course,
                isBusy: _controller.isCourseBusy(course.code),
                onAdd: () => _addCourse(course),
              ),
            );
          })
          .toList(growable: false),
    );
  }

  Widget _scheduledPanel(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        '待选课程',
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 3),
                      Text(
                        '仅保存在本机当前账号与学期分区。',
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
                ),
                IconButton(
                  tooltip: '清空本机课表',
                  onPressed: _controller.scheduled.isEmpty
                      ? null
                      : _confirmClear,
                  icon: const Icon(Icons.delete_sweep_outlined),
                ),
              ],
            ),
            const SizedBox(height: 12),
            if (_controller.scheduled.isEmpty)
              const _InlineEmpty(title: '还没有加入课程', message: '从左侧课程列表加入后会在这里显示。')
            else
              ..._controller.scheduled.map((ScheduledCourse scheduled) {
                return Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: _ScheduledCourseCard(
                    scheduled: scheduled,
                    onRemove: () => _removeCourse(scheduled.course.code),
                  ),
                );
              }),
          ],
        ),
      ),
    );
  }

  Widget _dropdown<T>({
    required String label,
    required String? selectedValue,
    required List<T> items,
    required String Function(T item) value,
    required String Function(T item) name,
    required ValueChanged<String?>? onChanged,
  }) {
    final bool hasSelected =
        selectedValue != null &&
        items.any((T item) => value(item) == selectedValue);
    return DropdownButtonFormField<String>(
      key: ValueKey<String>('$label:${hasSelected ? selectedValue : ''}'),
      initialValue: hasSelected ? selectedValue : null,
      isExpanded: true,
      decoration: InputDecoration(labelText: label),
      items: items
          .map((T item) {
            return DropdownMenuItem<String>(
              value: value(item),
              child: Text(name(item), overflow: TextOverflow.ellipsis),
            );
          })
          .toList(growable: false),
      onChanged: items.isEmpty ? null : onChanged,
    );
  }

  Future<void> _addCourse(SelectionCourse course) async {
    try {
      final ScheduleAddResult result = await _controller.addCourse(course);
      if (!mounted) {
        return;
      }
      switch (result.status) {
        case ScheduleAddStatus.added:
          _showMessage('${course.name} 已加入本机课表');
        case ScheduleAddStatus.duplicate:
          _showMessage('${course.name} 已在本机课表中');
        case ScheduleAddStatus.conflict:
          await _showConflict(result);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage(failure.message);
      }
    }
  }

  Future<void> _showConflict(ScheduleAddResult result) async {
    final ScheduleConflict conflict = result.conflict!;
    final String withName = conflict.withCourse.course.name;
    final String slot =
        '${_weekday(conflict.candidateSlot.weekday)} '
        '${conflict.candidateSlot.startSlot}–${conflict.candidateSlot.endSlot} 节';
    if (conflict.kind == ScheduleConflictKind.confirmed) {
      await showDialog<void>(
        context: context,
        builder: (BuildContext context) => AlertDialog(
          icon: const Icon(Icons.event_busy_outlined),
          title: const Text('课程时间冲突'),
          content: Text('该课程在 $slot 与“$withName”确认冲突，未加入课表。'),
          actions: <Widget>[
            FilledButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('知道了'),
            ),
          ],
        ),
      );
      return;
    }
    final bool? shouldAdd = await showDialog<bool>(
      context: context,
      builder: (BuildContext context) => AlertDialog(
        icon: const Icon(Icons.warning_amber_rounded),
        title: const Text('可能存在周次冲突'),
        content: Text(
          '该课程在 $slot 与“$withName”节次重叠，但至少一条时段的周次未知或无法解析。'
          '系统不能确认是否同周上课，是否仍要加入？',
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('仍要加入'),
          ),
        ],
      ),
    );
    if (shouldAdd != true || !mounted) {
      return;
    }
    try {
      await _controller.confirmAdd(
        result.pendingCourse!,
        result.pendingTimeslots!,
      );
      if (mounted) {
        _showMessage('${result.pendingCourse!.name} 已加入，并保留可能冲突提示');
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage(failure.message);
      }
    }
  }

  Future<void> _removeCourse(String courseCode) async {
    try {
      await _controller.removeCourse(courseCode);
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage(failure.message);
      }
    }
  }

  Future<void> _confirmClear() async {
    final bool? confirmed = await showDialog<bool>(
      context: context,
      builder: (BuildContext context) => AlertDialog(
        title: const Text('清空当前学期课表？'),
        content: const Text('只会删除本机当前账号、环境和学期分区中的课程，不会写回一系统。'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('清空'),
          ),
        ],
      ),
    );
    if (confirmed != true) {
      return;
    }
    try {
      await _controller.clearSchedule();
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage(failure.message);
      }
    }
  }

  void _showMessage(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }
}

class _ScopeNotice extends StatelessWidget {
  const _ScopeNotice();

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.secondaryContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: const Padding(
        padding: EdgeInsets.all(14),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Icon(Icons.info_outline_rounded),
            SizedBox(width: 10),
            Expanded(
              child: Text(
                '当前是课程级本机排课：不代表已选择具体教学班，不监测停开或换班，'
                '也不会跨设备同步或写回教务系统。',
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SelectionCourseRow extends StatelessWidget {
  const _SelectionCourseRow({
    required this.course,
    required this.isBusy,
    required this.onAdd,
  });

  final SelectionCourse course;
  final bool isBusy;
  final VoidCallback onAdd;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: Theme.of(context).colorScheme.outlineVariant),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: <Widget>[
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    course.name,
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    '${course.code} · ${_teachers(course)} · '
                    '${_number(course.credit)} 学分',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
            const SizedBox(width: 10),
            FilledButton.tonalIcon(
              onPressed: isBusy ? null : onAdd,
              icon: isBusy
                  ? const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.add_rounded),
              label: Text(isBusy ? '读取时段' : '加入'),
            ),
          ],
        ),
      ),
    );
  }
}

class _ScheduledCourseCard extends StatelessWidget {
  const _ScheduledCourseCard({required this.scheduled, required this.onRemove});

  final ScheduledCourse scheduled;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    scheduled.course.name,
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                ),
                IconButton(
                  tooltip: '移除 ${scheduled.course.name}',
                  onPressed: onRemove,
                  icon: const Icon(Icons.close_rounded),
                ),
              ],
            ),
            Text('${scheduled.course.code} · ${_teachers(scheduled.course)}'),
            const SizedBox(height: 6),
            if (scheduled.timeslots.isEmpty)
              const Text('课程镜像暂无时段，无法执行冲突检查。')
            else
              ...scheduled.timeslots.map((TimeSlot timeslot) {
                return Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: Text(
                    '${_weekday(timeslot.weekday)} '
                    '${timeslot.startSlot}–${timeslot.endSlot} 节 · '
                    '${timeslot.weeks?.trim().isNotEmpty == true ? timeslot.weeks : '周次未知'}'
                    '${timeslot.location?.trim().isNotEmpty == true ? ' · ${timeslot.location}' : ''}',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                );
              }),
            if (scheduled.hasUnknownWeeks) ...<Widget>[
              const SizedBox(height: 7),
              Text(
                '周次未知：与同日重叠节次按可能冲突提示。',
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class ScheduleTimetable extends StatelessWidget {
  const ScheduleTimetable({required this.courses, super.key});

  final List<ScheduledCourse> courses;

  @override
  Widget build(BuildContext context) {
    if (courses.isEmpty) {
      return const _InlineEmpty(
        title: '课表为空',
        message: '加入课程后会按周一至周日、1–13 节显示。',
      );
    }
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        if (constraints.maxWidth < 700) {
          return _CompactTimetable(courses: courses);
        }
        return _ExpandedTimetable(courses: courses);
      },
    );
  }
}

class _CompactTimetable extends StatefulWidget {
  const _CompactTimetable({required this.courses});

  final List<ScheduledCourse> courses;

  @override
  State<_CompactTimetable> createState() => _CompactTimetableState();
}

class _CompactTimetableState extends State<_CompactTimetable> {
  int _selectedWeekday = 1;

  @override
  Widget build(BuildContext context) {
    final List<({ScheduledCourse course, TimeSlot slot})> entries =
        widget.courses
            .expand(
              (ScheduledCourse course) => course.timeslots
                  .where((TimeSlot slot) => slot.weekday == _selectedWeekday)
                  .map((TimeSlot slot) => (course: course, slot: slot)),
            )
            .toList()
          ..sort(
            (
              ({ScheduledCourse course, TimeSlot slot}) left,
              ({ScheduledCourse course, TimeSlot slot}) right,
            ) => left.slot.startSlot.compareTo(right.slot.startSlot),
          );
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: List<Widget>.generate(7, (int index) {
                  final int weekday = index + 1;
                  return Padding(
                    padding: const EdgeInsets.only(right: 6),
                    child: ChoiceChip(
                      label: Text(_weekday(weekday)),
                      selected: _selectedWeekday == weekday,
                      onSelected: (_) =>
                          setState(() => _selectedWeekday = weekday),
                    ),
                  );
                }),
              ),
            ),
            const SizedBox(height: 12),
            if (entries.isEmpty)
              const Padding(
                padding: EdgeInsets.all(20),
                child: Text('这一天没有课程。', textAlign: TextAlign.center),
              )
            else
              ...entries.map((entry) {
                return ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: CircleAvatar(child: Text('${entry.slot.startSlot}')),
                  title: Text(entry.course.course.name),
                  subtitle: Text(
                    '${entry.slot.startSlot}–${entry.slot.endSlot} 节 · '
                    '${entry.slot.weeks?.trim().isNotEmpty == true ? entry.slot.weeks : '周次未知'}'
                    '${entry.slot.location?.trim().isNotEmpty == true ? ' · ${entry.slot.location}' : ''}',
                  ),
                );
              }),
          ],
        ),
      ),
    );
  }
}

class _ExpandedTimetable extends StatelessWidget {
  const _ExpandedTimetable({required this.courses});

  final List<ScheduledCourse> courses;

  @override
  Widget build(BuildContext context) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: SizedBox(
          width: 900,
          child: Table(
            border: TableBorder.all(
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
            columnWidths: const <int, TableColumnWidth>{
              0: FixedColumnWidth(52),
            },
            children: <TableRow>[
              TableRow(
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.surfaceContainerHighest,
                ),
                children: <Widget>[
                  const _TableHeader(label: '节次'),
                  ...List<Widget>.generate(
                    7,
                    (int index) => _TableHeader(label: _weekday(index + 1)),
                  ),
                ],
              ),
              ...List<TableRow>.generate(13, (int slotIndex) {
                final int section = slotIndex + 1;
                return TableRow(
                  children: <Widget>[
                    _TableHeader(label: '$section'),
                    ...List<Widget>.generate(7, (int weekdayIndex) {
                      final int weekday = weekdayIndex + 1;
                      final List<ScheduledCourse> matches = courses
                          .where(
                            (ScheduledCourse course) => course.timeslots.any(
                              (TimeSlot slot) =>
                                  slot.weekday == weekday &&
                                  section >= slot.startSlot &&
                                  section <= slot.endSlot,
                            ),
                          )
                          .toList(growable: false);
                      return Container(
                        constraints: const BoxConstraints(minHeight: 68),
                        padding: const EdgeInsets.all(4),
                        child: Column(
                          children: matches
                              .map((ScheduledCourse item) {
                                return Padding(
                                  padding: const EdgeInsets.only(bottom: 3),
                                  child: DecoratedBox(
                                    decoration: BoxDecoration(
                                      color: Theme.of(
                                        context,
                                      ).colorScheme.primaryContainer,
                                      borderRadius: BorderRadius.circular(6),
                                    ),
                                    child: Padding(
                                      padding: const EdgeInsets.all(5),
                                      child: Text(
                                        item.course.name,
                                        style: Theme.of(
                                          context,
                                        ).textTheme.labelSmall,
                                      ),
                                    ),
                                  ),
                                );
                              })
                              .toList(growable: false),
                        ),
                      );
                    }),
                  ],
                );
              }),
            ],
          ),
        ),
      ),
    );
  }
}

class _TableHeader extends StatelessWidget {
  const _TableHeader({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 10),
      child: Text(
        label,
        textAlign: TextAlign.center,
        style: Theme.of(context).textTheme.labelSmall,
      ),
    );
  }
}

class _InlineEmpty extends StatelessWidget {
  const _InlineEmpty({required this.title, required this.message});

  final String title;
  final String message;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        border: Border.all(color: Theme.of(context).colorScheme.outlineVariant),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          children: <Widget>[
            const Icon(Icons.inbox_outlined),
            const SizedBox(height: 8),
            Text(title, style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text(message, textAlign: TextAlign.center),
          ],
        ),
      ),
    );
  }
}

class _InlineFailure extends StatelessWidget {
  const _InlineFailure({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.errorContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: <Widget>[
            const Icon(Icons.error_outline_rounded),
            const SizedBox(width: 8),
            Expanded(child: Text(message)),
          ],
        ),
      ),
    );
  }
}

String _weekday(int weekday) {
  return const <String>[
    '周一',
    '周二',
    '周三',
    '周四',
    '周五',
    '周六',
    '周日',
  ][weekday.clamp(1, 7) - 1];
}

String _teachers(SelectionCourse course) {
  if (course.teacherNames.isNotEmpty) {
    return course.teacherNames.join(' / ');
  }
  return course.teacherName?.trim().isNotEmpty == true
      ? course.teacherName!
      : '教师待同步';
}

String _number(num? value) {
  if (value == null) {
    return '0';
  }
  return value % 1 == 0 ? value.toInt().toString() : value.toStringAsFixed(1);
}

String _formatDateTime(DateTime? date) {
  if (date == null) {
    return '待同步';
  }
  final DateTime local = date.toLocal();
  return '${local.year}-${local.month.toString().padLeft(2, '0')}-'
      '${local.day.toString().padLeft(2, '0')} '
      '${local.hour.toString().padLeft(2, '0')}:'
      '${local.minute.toString().padLeft(2, '0')}';
}
