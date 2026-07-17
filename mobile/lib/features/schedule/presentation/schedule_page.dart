import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
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
  int? _draftWeekday;
  int? _draftStartSlot;
  int? _draftEndSlot;
  int? _draftWeek;
  bool _draftIncludeUnknownSchedule = true;

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
    final LatestUpdate? latest = _controller.latestUpdate;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text('教学班排课', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 6),
        Text(
          '浏览具体教学班，完成本机待选、周次提示与冲突检查。',
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          '上游课程最近更新：${_formatDateTime(latest?.updatedAt)}',
          style: Theme.of(context).textTheme.bodySmall,
        ),
        const SizedBox(height: 2),
        Text(
          '当前镜像导入：${_formatDateTime(latest?.importedAt)}'
          '${latest?.stale == true ? ' · 数据可能已过期' : ''}',
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: latest?.stale == true
                ? Theme.of(context).colorScheme.error
                : null,
          ),
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
                    value: (Calendar item) => item.id,
                    name: (Calendar item) => item.name,
                    onChanged: _selectCalendar,
                  ),
                  _dropdown<String>(
                    label: '年级',
                    selectedValue: _controller.grade,
                    items: _controller.grades,
                    value: (String item) => item,
                    name: (String item) => item,
                    onChanged:
                        _controller.calendarId == null ||
                            _controller.areContextOptionsLoading
                        ? null
                        : _controller.selectGrade,
                  ),
                  _dropdown<Major>(
                    label: '专业',
                    selectedValue: _controller.majorId,
                    items: _controller.majors,
                    value: (Major item) => item.id,
                    name: (Major item) => item.name,
                    onChanged:
                        _controller.grade == null ||
                            _controller.areContextOptionsLoading
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
                value: (CourseNature item) => item.id,
                name: (CourseNature item) => item.name,
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
            const SizedBox(height: 10),
            _timeFilters(context),
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
        _controller.offeringsFailure == null) {
      return const _InlineEmpty(title: '输入关键词搜索', message: '至少输入 2 个字符。');
    }
    if (_controller.areOfferingsLoading) {
      return const Padding(
        padding: EdgeInsets.symmetric(vertical: 36),
        child: Center(child: CircularProgressIndicator()),
      );
    }
    if (_controller.offeringsFailure case final ApiFailure failure
        when _controller.offerings.isEmpty) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          _InlineFailure(message: failure.message),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: _controller.retryOfferings,
            icon: const Icon(Icons.refresh_rounded),
            label: const Text('重试'),
          ),
        ],
      );
    }
    if (_controller.offerings.isEmpty) {
      return const _InlineEmpty(title: '没有找到教学班', message: '更换筛选条件或关键词后再试。');
    }
    return Column(
      children: <Widget>[
        if (_controller.offeringsFailure
            case final ApiFailure failure) ...<Widget>[
          _InlineFailure(message: failure.message),
          const SizedBox(height: 8),
        ],
        ..._controller.offerings.map((SelectionOffering offering) {
          return Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: _SelectionOfferingRow(
              offering: offering,
              isBusy: _controller.isOfferingBusy(offering.offeringId),
              onAdd: () => _addOffering(offering),
            ),
          );
        }),
        if (_controller.hasMore)
          OutlinedButton.icon(
            onPressed: _controller.isLoadingMore ? null : _controller.loadMore,
            icon: _controller.isLoadingMore
                ? const SizedBox.square(
                    dimension: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.expand_more_rounded),
            label: Text(_controller.isLoadingMore ? '加载中' : '加载更多教学班'),
          ),
      ],
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
                        '待选教学班',
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
                  tooltip: '导出或分享课表 JSON',
                  onPressed: _controller.scheduled.isEmpty
                      ? null
                      : _copyScheduleJson,
                  icon: const Icon(Icons.ios_share_outlined),
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
              const _InlineEmpty(title: '还没有加入教学班', message: '从教学班列表加入后会在这里显示。')
            else
              ..._controller.scheduled.map((ScheduledCourse scheduled) {
                return Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: _ScheduledCourseCard(
                    scheduled: scheduled,
                    onRemove: () =>
                        _removeOffering(scheduled.offering.offeringId),
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

  Widget _timeFilters(BuildContext context) {
    final bool hasAppliedFilter =
        _controller.weekday != null ||
        _controller.startSlot != null ||
        _controller.endSlot != null ||
        _controller.week != null ||
        !_controller.includeUnknownSchedule;
    final bool hasDraftFilter =
        _draftWeekday != null ||
        _draftStartSlot != null ||
        _draftEndSlot != null ||
        _draftWeek != null ||
        !_draftIncludeUnknownSchedule;
    final bool hasCompleteSlotRange =
        _draftWeekday != null &&
        _draftStartSlot != null &&
        _draftEndSlot != null;
    final bool hasPartialSlotRange =
        <int?>[
          _draftWeekday,
          _draftStartSlot,
          _draftEndSlot,
        ].any((int? value) => value != null) &&
        !hasCompleteSlotRange;
    final bool excludesUnknownWithoutRange =
        !_draftIncludeUnknownSchedule && !hasCompleteSlotRange;
    final bool canApply =
        !hasPartialSlotRange &&
        !excludesUnknownWithoutRange &&
        (_draftWeek == null || hasCompleteSlotRange) &&
        (_draftStartSlot == null ||
            _draftEndSlot == null ||
            _draftStartSlot! <= _draftEndSlot!);
    return ExpansionTile(
      tilePadding: EdgeInsets.zero,
      childrenPadding: EdgeInsets.zero,
      title: const Text('按空闲时间筛选'),
      subtitle: Text(
        hasPartialSlotRange ||
                _draftWeek != null && !hasCompleteSlotRange ||
                excludesUnknownWithoutRange
            ? '星期、起始节次和结束节次需要一起选择'
            : hasAppliedFilter
            ? '已应用时间条件'
            : '默认保留排课未知的教学班',
      ),
      children: <Widget>[
        LayoutBuilder(
          builder: (BuildContext context, BoxConstraints constraints) {
            final List<Widget> fields = <Widget>[
              _integerFilter(
                label: '星期',
                selectedValue: _draftWeekday,
                values: List<int>.generate(7, (int index) => index + 1),
                name: _weekday,
                onChanged: (int? value) =>
                    setState(() => _draftWeekday = value),
              ),
              _integerFilter(
                label: '起始节次',
                selectedValue: _draftStartSlot,
                values: List<int>.generate(20, (int index) => index + 1),
                name: (int value) => '$value 节',
                onChanged: (int? value) =>
                    setState(() => _draftStartSlot = value),
              ),
              _integerFilter(
                label: '结束节次',
                selectedValue: _draftEndSlot,
                values: List<int>.generate(20, (int index) => index + 1),
                name: (int value) => '$value 节',
                onChanged: (int? value) =>
                    setState(() => _draftEndSlot = value),
              ),
              _integerFilter(
                label: '周次',
                selectedValue: _draftWeek,
                values: List<int>.generate(30, (int index) => index + 1),
                name: (int value) => '第 $value 周',
                onChanged: (int? value) => setState(() => _draftWeek = value),
              ),
            ];
            if (constraints.maxWidth < 620) {
              return Column(
                children:
                    fields
                        .expand(
                          (Widget field) => <Widget>[
                            field,
                            const SizedBox(height: 10),
                          ],
                        )
                        .toList()
                      ..removeLast(),
              );
            }
            return Wrap(
              spacing: 10,
              runSpacing: 10,
              children: fields
                  .map((Widget field) => SizedBox(width: 150, child: field))
                  .toList(growable: false),
            );
          },
        ),
        SwitchListTile.adaptive(
          contentPadding: EdgeInsets.zero,
          title: const Text('保留排课未知教学班'),
          subtitle: const Text('关闭后只显示能确认满足空闲条件的教学班。'),
          value: _draftIncludeUnknownSchedule,
          onChanged: hasCompleteSlotRange || !_draftIncludeUnknownSchedule
              ? (bool value) =>
                    setState(() => _draftIncludeUnknownSchedule = value)
              : null,
        ),
        Row(
          mainAxisAlignment: MainAxisAlignment.end,
          children: <Widget>[
            if (hasDraftFilter || hasAppliedFilter)
              TextButton.icon(
                onPressed: _clearTimeFilters,
                icon: const Icon(Icons.filter_alt_off_outlined),
                label: const Text('清除'),
              ),
            const SizedBox(width: 8),
            FilledButton.icon(
              onPressed: canApply ? _applyTimeFilters : null,
              icon: const Icon(Icons.filter_alt_outlined),
              label: const Text('应用筛选'),
            ),
          ],
        ),
      ],
    );
  }

  Widget _integerFilter({
    required String label,
    required int? selectedValue,
    required List<int> values,
    required String Function(int value) name,
    required ValueChanged<int?> onChanged,
  }) {
    return DropdownButtonFormField<int>(
      key: ValueKey<String>('$label:${selectedValue ?? ''}'),
      initialValue: selectedValue,
      isExpanded: true,
      decoration: InputDecoration(labelText: label),
      items: <DropdownMenuItem<int>>[
        const DropdownMenuItem<int>(value: null, child: Text('不限')),
        ...values.map(
          (int value) =>
              DropdownMenuItem<int>(value: value, child: Text(name(value))),
        ),
      ],
      onChanged: onChanged,
    );
  }

  Future<void> _selectCalendar(String? calendarId) async {
    _searchController.clear();
    _resetDraftTimeFilters();
    await _controller.selectCalendar(calendarId);
  }

  Future<void> _applyTimeFilters() async {
    try {
      await _controller.updateTimeFilters(
        weekday: _draftWeekday,
        startSlot: _draftStartSlot,
        endSlot: _draftEndSlot,
        week: _draftWeek,
        includeUnknownSchedule: _draftIncludeUnknownSchedule,
      );
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage(failure.message);
      }
    }
  }

  Future<void> _clearTimeFilters() async {
    _resetDraftTimeFilters();
    await _controller.clearTimeFilters();
  }

  void _resetDraftTimeFilters() {
    setState(() {
      _draftWeekday = null;
      _draftStartSlot = null;
      _draftEndSlot = null;
      _draftWeek = null;
      _draftIncludeUnknownSchedule = true;
    });
  }

  Future<void> _addOffering(SelectionOffering offering) async {
    try {
      final ScheduleAddResult result = await _controller.addOffering(offering);
      if (!mounted) {
        return;
      }
      switch (result.status) {
        case ScheduleAddStatus.added:
          _showMessage('${offering.name} 已加入本机课表');
        case ScheduleAddStatus.duplicate:
          _showMessage('${offering.name} 已在本机课表中');
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
    final String withName = conflict.withCourse.offering.name;
    final TimeSlot? candidateSlot = conflict.candidateSlot;
    final String slot = candidateSlot == null
        ? '未知排课时段'
        : '${_weekday(candidateSlot.weekday)} '
              '${candidateSlot.startSlot}–${candidateSlot.endSlot} 节';
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
          candidateSlot == null
              ? '该教学班排课完全未知，无法排除与“$withName”冲突。是否仍要加入？'
              : '该课程在 $slot 与“$withName”节次重叠，但至少一条时段的周次未知或无法解析。'
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
        result.pendingOffering!,
        result.pendingTimeslots!,
      );
      if (mounted) {
        _showMessage('${result.pendingOffering!.name} 已加入，并保留可能冲突提示');
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage(failure.message);
      }
    }
  }

  Future<void> _removeOffering(String offeringId) async {
    try {
      await _controller.removeOffering(offeringId);
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
        content: const Text(
          '只会删除本机当前账号、环境和学期分区中的课程，'
          '不会写回一系统。',
        ),
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

  Future<void> _copyScheduleJson() async {
    final bool? confirmed = await showDialog<bool>(
      context: context,
      builder: (BuildContext context) => AlertDialog(
        icon: const Icon(Icons.privacy_tip_outlined),
        title: const Text('复制课表 JSON？'),
        content: const Text(
          '导出包含 API 环境、当前学期、教学班与时段，用于导入时防止串环境。'
          '不包含账号标识或登录凭据。'
          '复制到系统剪贴板后，其他应用可能读取；请只粘贴给信任的人。',
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton.icon(
            onPressed: () => Navigator.of(context).pop(true),
            icon: const Icon(Icons.content_copy_rounded),
            label: const Text('复制 JSON'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    try {
      final String payload = _controller.exportScheduleJson();
      await Clipboard.setData(ClipboardData(text: payload));
      if (mounted) {
        _showMessage('课表 JSON 已复制，可粘贴到你信任的应用中分享');
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage(failure.message);
      }
    } on Object {
      if (mounted) {
        _showMessage('系统剪贴板暂不可用，课表未导出');
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
                '当前按具体教学班在本机排课；这不是官方选课结果，不监测停开或换班，'
                '也不会跨设备同步或写回教务系统。',
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SelectionOfferingRow extends StatelessWidget {
  const _SelectionOfferingRow({
    required this.offering,
    required this.isBusy,
    required this.onAdd,
  });

  final SelectionOffering offering;
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
                    offering.name,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    '${offering.code} · ${_teachingClassLabel(offering)} · '
                    '${_number(offering.credit)} 学分',
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                  const SizedBox(height: 3),
                  Text(
                    '${_teachers(offering)} · ${_weeksLabel(offering)}'
                    '${offering.scheduleUnknown ? ' · 排课待同步' : ''}',
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                  const SizedBox(height: 3),
                  Text(
                    _reviewLabel(offering),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
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
                    scheduled.offering.name,
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                ),
                IconButton(
                  tooltip: '移除 ${scheduled.offering.name}',
                  onPressed: onRemove,
                  icon: const Icon(Icons.close_rounded),
                ),
              ],
            ),
            Text(
              '${scheduled.offering.code} · '
              '${_teachingClassLabel(scheduled.offering)} · '
              '${_teachers(scheduled.offering)}',
            ),
            const SizedBox(height: 6),
            if (scheduled.timeslots.isEmpty)
              Text(
                scheduled.hasUnknownSchedule
                    ? '该教学班排课未知，加入时按可能冲突处理。'
                    : '该教学班当前无上课时段。',
              )
            else
              ...scheduled.timeslots.map((TimeSlot timeslot) {
                return Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: Text(
                    '${_weekday(timeslot.weekday)} '
                    '${timeslot.startSlot}–${timeslot.endSlot} 节 · '
                    '${_timeslotWeeks(timeslot)}'
                    '${timeslot.locationUnknown
                        ? ' · 地点未知'
                        : timeslot.location?.trim().isNotEmpty == true
                        ? ' · ${timeslot.location}'
                        : ''}',
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
        message: '加入课程后会按周一至周日、1–20 节显示。',
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
                  title: Text(entry.course.offering.name),
                  subtitle: Text(
                    '${entry.slot.startSlot}–${entry.slot.endSlot} 节 · '
                    '${_timeslotWeeks(entry.slot)}'
                    '${entry.slot.locationUnknown
                        ? ' · 地点未知'
                        : entry.slot.location?.trim().isNotEmpty == true
                        ? ' · ${entry.slot.location}'
                        : ''}',
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
              ...List<TableRow>.generate(20, (int slotIndex) {
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
                                        item.offering.name,
                                        maxLines: 3,
                                        overflow: TextOverflow.ellipsis,
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

String _teachers(SelectionOffering offering) {
  if (offering.teacherNames.isNotEmpty) {
    return offering.teacherNames.join(' / ');
  }
  return offering.teacherName?.trim().isNotEmpty == true
      ? offering.teacherName!
      : '教师待同步';
}

String _teachingClassLabel(SelectionOffering offering) {
  final String? teachingClassCode = offering.teachingClassCode?.trim();
  return teachingClassCode?.isNotEmpty == true
      ? '教学班 $teachingClassCode'
      : '教学班 ${offering.offeringId}';
}

String _weeksLabel(SelectionOffering offering) {
  if (offering.weeksUnknown) {
    return '周次未知';
  }
  if (offering.startWeek != null && offering.endWeek != null) {
    return '${offering.startWeek}–${offering.endWeek} 周';
  }
  return '周次见时段';
}

String _reviewLabel(SelectionOffering offering) {
  if (offering.reviewCount == 0) {
    return '暂无历史评分';
  }
  final num? reviewAvg = offering.reviewAvg;
  if (reviewAvg == null) {
    return '历史评分数据待同步';
  }
  final String scope = switch (offering.reviewScope) {
    SelectionOfferingReviewScopeEnum.teacher => '当前教师',
    SelectionOfferingReviewScopeEnum.course => '课程参考',
    SelectionOfferingReviewScopeEnum.none ||
    SelectionOfferingReviewScopeEnum.unknownDefaultOpenApi => '评分口径待更新',
  };
  return '${reviewAvg.toStringAsFixed(1)} 分 · ${offering.reviewCount} 条历史评课 · $scope';
}

String _timeslotWeeks(TimeSlot timeslot) {
  if (timeslot.weeksUnknown || timeslot.weekNumbers.isEmpty) {
    return '周次未知';
  }
  if (timeslot.weeks?.trim().isNotEmpty == true) {
    return timeslot.weeks!;
  }
  final List<int> weeks = timeslot.weekNumbers.toList()..sort();
  return '${weeks.join(',')} 周';
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
