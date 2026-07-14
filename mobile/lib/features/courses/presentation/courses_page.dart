import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../data/courses_repository.dart';
import '../domain/course_catalog_controller.dart';
import '../domain/course_models.dart';

class CoursesPage extends ConsumerStatefulWidget {
  const CoursesPage({super.key});

  @override
  ConsumerState<CoursesPage> createState() => _CoursesPageState();
}

class _CoursesPageState extends ConsumerState<CoursesPage> {
  late final CourseCatalogController _controller;
  final TextEditingController _queryController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _controller = CourseCatalogController(ref.read(coursesRepositoryProvider));
    _controller.initialize();
  }

  @override
  void dispose() {
    _queryController.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: _controller,
      builder: (BuildContext context, Widget? child) {
        return CustomScrollView(
          key: const PageStorageKey<String>('courses-catalog'),
          slivers: <Widget>[
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(16, 20, 16, 12),
              sliver: SliverToBoxAdapter(child: _header(context)),
            ),
            SliverPadding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              sliver: SliverToBoxAdapter(child: _filters(context)),
            ),
            if (_controller.departmentsFailure case final ApiFailure failure)
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                sliver: SliverToBoxAdapter(
                  child: _InlineWarning(message: '院系列表暂不可用：${failure.message}'),
                ),
              ),
            ..._content(context),
            const SliverPadding(padding: EdgeInsets.only(bottom: 32)),
          ],
        );
      },
    );
  }

  Widget _header(BuildContext context) {
    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 1040),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text('课程点评', style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 6),
          Text(
            '浏览课程、查看点评统计与 AI 摘要。',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }

  Widget _filters(BuildContext context) {
    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 1040),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          const SizedBox(height: 16),
          LayoutBuilder(
            builder: (BuildContext context, BoxConstraints constraints) {
              final Widget search = TextField(
                controller: _queryController,
                textInputAction: TextInputAction.search,
                decoration: InputDecoration(
                  labelText: '搜索课程',
                  hintText: '课程名、课号、教师或拼音',
                  prefixIcon: const Icon(Icons.search_rounded),
                  suffixIcon: _controller.query.isEmpty
                      ? null
                      : IconButton(
                          tooltip: '清除搜索',
                          onPressed: () {
                            _queryController.clear();
                            _controller.submitQuery('');
                          },
                          icon: const Icon(Icons.close_rounded),
                        ),
                ),
                onSubmitted: _controller.submitQuery,
              );
              final Widget searchButton = FilledButton.icon(
                onPressed: () => _controller.submitQuery(_queryController.text),
                icon: const Icon(Icons.search_rounded),
                label: const Text('搜索'),
              );
              if (constraints.maxWidth < 640) {
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    search,
                    const SizedBox(height: 10),
                    searchButton,
                  ],
                );
              }
              return Row(
                children: <Widget>[
                  Expanded(child: search),
                  const SizedBox(width: 12),
                  searchButton,
                ],
              );
            },
          ),
          const SizedBox(height: 14),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: <Widget>[
              SizedBox(
                width: 240,
                child: DropdownButtonFormField<String>(
                  key: ValueKey<String?>(_controller.departmentId),
                  initialValue: _controller.departmentId ?? '',
                  isExpanded: true,
                  decoration: const InputDecoration(labelText: '院系'),
                  items: <DropdownMenuItem<String>>[
                    const DropdownMenuItem<String>(
                      value: '',
                      child: Text('全部院系'),
                    ),
                    ..._controller.departments.map((Department department) {
                      return DropdownMenuItem<String>(
                        value: department.id!,
                        child: Text(
                          department.name!,
                          overflow: TextOverflow.ellipsis,
                        ),
                      );
                    }),
                  ],
                  onChanged: _controller.query.isNotEmpty
                      ? null
                      : _controller.setDepartment,
                ),
              ),
              SegmentedButton<String>(
                segments: const <ButtonSegment<String>>[
                  ButtonSegment<String>(value: 'hot', label: Text('热门')),
                  ButtonSegment<String>(value: 'rating', label: Text('评分')),
                  ButtonSegment<String>(value: 'new', label: Text('最新')),
                ],
                selected: <String>{_controller.sort},
                onSelectionChanged: _controller.query.isNotEmpty
                    ? null
                    : (Set<String> values) {
                        _controller.setSort(values.first);
                      },
              ),
            ],
          ),
          if (_controller.query.isNotEmpty) ...<Widget>[
            const SizedBox(height: 10),
            Text(
              '正在搜索“${_controller.query}”；院系与排序仅用于浏览列表。',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ],
      ),
    );
  }

  List<Widget> _content(BuildContext context) {
    if (_controller.isLoading) {
      return const <Widget>[
        SliverFillRemaining(
          hasScrollBody: false,
          child: AppLoadingState(title: '正在加载课程', description: '正在读取课程与点评统计。'),
        ),
      ];
    }
    if (_controller.failure case final ApiFailure failure) {
      if (_controller.courses.isEmpty) {
        return <Widget>[
          SliverFillRemaining(
            hasScrollBody: false,
            child: AppErrorState(
              description: failure.message,
              onRetry: _controller.reload,
            ),
          ),
        ];
      }
    }
    if (_controller.courses.isEmpty) {
      return const <Widget>[
        SliverFillRemaining(
          hasScrollBody: false,
          child: AppEmptyState(title: '没有找到课程', description: '换一个关键词、院系或排序再试。'),
        ),
      ];
    }
    return <Widget>[
      SliverPadding(
        padding: const EdgeInsets.fromLTRB(16, 18, 16, 0),
        sliver: SliverToBoxAdapter(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 1040),
            child: LayoutBuilder(
              builder: (BuildContext context, BoxConstraints constraints) {
                final double itemWidth = constraints.maxWidth < 720
                    ? constraints.maxWidth
                    : (constraints.maxWidth - 16) / 2;
                return Wrap(
                  spacing: 16,
                  runSpacing: 16,
                  children: _controller.courses
                      .map((CourseListEntry course) {
                        return SizedBox(
                          width: itemWidth,
                          child: _CourseCard(course: course),
                        );
                      })
                      .toList(growable: false),
                );
              },
            ),
          ),
        ),
      ),
      if (_controller.failure case final ApiFailure failure)
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(16, 14, 16, 0),
          sliver: SliverToBoxAdapter(
            child: _InlineWarning(message: '继续加载失败：${failure.message}'),
          ),
        ),
      if (_controller.hasMore)
        SliverPadding(
          padding: const EdgeInsets.fromLTRB(16, 18, 16, 0),
          sliver: SliverToBoxAdapter(
            child: Center(
              child: OutlinedButton.icon(
                onPressed: _controller.isLoadingMore
                    ? null
                    : _controller.loadMore,
                icon: _controller.isLoadingMore
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.expand_more_rounded),
                label: Text(_controller.isLoadingMore ? '正在加载' : '加载更多课程'),
              ),
            ),
          ),
        ),
    ];
  }
}

class _CourseCard extends StatelessWidget {
  const _CourseCard({required this.course});

  final CourseListEntry course;

  @override
  Widget build(BuildContext context) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: () => context.push('/courses/${Uri.encodeComponent(course.id)}'),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Expanded(
                    child: Text(
                      course.name,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ),
                  const SizedBox(width: 12),
                  _Pill(label: '${_number(course.credit)} 学分'),
                ],
              ),
              const SizedBox(height: 8),
              Text(
                '${course.code.isEmpty ? '课号待同步' : course.code} · '
                '${course.teacherName?.trim().isNotEmpty == true ? course.teacherName : '教师待同步'}',
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 14),
              Row(
                children: <Widget>[
                  Icon(
                    Icons.star_rounded,
                    size: 20,
                    color: Theme.of(context).colorScheme.primary,
                  ),
                  const SizedBox(width: 4),
                  Text(_rating(course.reviewAverage)),
                  const SizedBox(width: 12),
                  Text('${course.reviewCount} 条点评'),
                  const Spacer(),
                  const Icon(Icons.chevron_right_rounded),
                ],
              ),
              if (course.department?.trim().isNotEmpty == true) ...<Widget>[
                const SizedBox(height: 10),
                Text(
                  course.department!,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _Pill extends StatelessWidget {
  const _Pill({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.secondaryContainer,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
        child: Text(label, style: Theme.of(context).textTheme.labelMedium),
      ),
    );
  }
}

class _InlineWarning extends StatelessWidget {
  const _InlineWarning({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      liveRegion: true,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Theme.of(context).colorScheme.errorContainer,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: <Widget>[
              const Icon(Icons.warning_amber_rounded),
              const SizedBox(width: 10),
              Expanded(child: Text(message)),
            ],
          ),
        ),
      ),
    );
  }
}

String _number(num? value) {
  if (value == null) {
    return '0';
  }
  return value % 1 == 0 ? value.toInt().toString() : value.toStringAsFixed(1);
}

String _rating(num? value) => value == null ? '暂无评分' : value.toStringAsFixed(1);
