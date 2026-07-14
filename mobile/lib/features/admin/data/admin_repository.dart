import 'package:dio/dio.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../domain/admin_capabilities.dart';
import '../domain/admin_mutations.dart';

class AdminActorContext {
  const AdminActorContext({
    required this.accountId,
    required this.role,
    required this.capabilities,
  });

  final String accountId;
  final String role;
  final Set<String> capabilities;

  bool canManageTarget({required String accountId, required String role}) {
    if (accountId == this.accountId) {
      return false;
    }
    const Map<String, int> ranks = <String, int>{
      'user': 0,
      'mod': 1,
      'admin': 2,
    };
    final int? actorRank = ranks[this.role];
    final int? targetRank = ranks[role];
    return actorRank != null && targetRank != null && targetRank < actorRank;
  }
}

class AdminAccessDenied implements Exception {
  const AdminAccessDenied();

  @override
  String toString() => '当前账号没有访问此管理模块的能力';
}

class AdminRecentAuthenticationRequired implements Exception {
  const AdminRecentAuthenticationRequired();

  @override
  String toString() => '服务器要求先完成近期身份验证';
}

class AdminRecord {
  const AdminRecord({
    required this.id,
    required this.title,
    required this.subtitle,
    this.evidence = const <String>[],
    this.actions = const <AdminMutationAction>[],
  });

  final String id;
  final String title;
  final String subtitle;
  final List<String> evidence;
  final List<AdminMutationAction> actions;
}

class AdminRecordGroup {
  const AdminRecordGroup({
    required this.title,
    required this.records,
    this.description,
    this.hasMore = false,
  });

  final String title;
  final String? description;
  final List<AdminRecord> records;
  final bool hasMore;
}

class AdminSectionSnapshot {
  const AdminSectionSnapshot({
    required this.section,
    required this.groups,
    required this.loadedAt,
    this.actions = const <AdminMutationAction>[],
  });

  final AdminSection section;
  final List<AdminRecordGroup> groups;
  final DateTime loadedAt;
  final List<AdminMutationAction> actions;
}

abstract interface class AdminReadDataSource {
  Future<AdminSectionSnapshot> load(
    AdminSection section,
    AdminActorContext actor,
  );
}

class AdminRepository {
  const AdminRepository(this._dataSource);

  final AdminReadDataSource _dataSource;

  Future<AdminSectionSnapshot> load(
    AdminSection section,
    AdminActorContext actor,
  ) async {
    final AdminModule? module = adminModuleForSection(section);
    if (module == null || !module.isVisibleTo(actor.capabilities)) {
      throw const AdminAccessDenied();
    }
    try {
      return await _dataSource.load(section, actor);
    } on DioException catch (error) {
      if (error.response?.statusCode == 428 ||
          _errorCode(error.response?.data) == 'RECENT_AUTH_REQUIRED') {
        throw const AdminRecentAuthenticationRequired();
      }
      throw ApiFailure.fromDio(error);
    }
  }

  static String? _errorCode(Object? data) {
    if (data is! Map) {
      return null;
    }
    final Object? error = data['error'];
    if (error is! Map) {
      return null;
    }
    final Object? code = error['code'];
    return code is String ? code : null;
  }
}

class GeneratedAdminReadDataSource implements AdminReadDataSource {
  const GeneratedAdminReadDataSource(this._api);

  final AdminApi _api;

  @override
  Future<AdminSectionSnapshot> load(
    AdminSection section,
    AdminActorContext actor,
  ) async {
    final Set<String> capabilities = actor.capabilities;
    final List<AdminRecordGroup> groups = switch (section) {
      AdminSection.overview => await _overview(),
      AdminSection.users => await _users(actor),
      AdminSection.moderation => await _moderation(capabilities),
      AdminSection.appeals => await _appeals(),
      AdminSection.resources => await _resources(capabilities),
      AdminSection.activity => await _activity(),
      AdminSection.announcements => await _announcements(),
      AdminSection.promotions => await _promotions(),
      AdminSection.achievements => await _achievements(),
      AdminSection.verifications => await _verifications(),
      AdminSection.creditIntegrity => await _creditIntegrity(),
      AdminSection.audit => await _audit(),
      AdminSection.system => await _system(capabilities),
    };
    return AdminSectionSnapshot(
      section: section,
      groups: groups,
      loadedAt: DateTime.now(),
      actions: _sectionActions(section, actor),
    );
  }

  Future<List<AdminRecordGroup>> _overview() async {
    final AdminOverview overview = _required(
      await _api.adminOverviewGet(),
      '管理概览',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '社区状态',
        records: <AdminRecord>[
          _metric(
            'users',
            '用户',
            overview.totalUsers,
            '活跃 ${overview.activeUsers}',
          ),
          _metric(
            'accounts',
            '受限账号',
            overview.suspendedUsers,
            '版主 ${overview.moderators ?? 0} · 管理员 ${overview.administrators ?? 0}',
          ),
          _metric(
            'forum',
            '今日社区',
            overview.threadsToday,
            '主题 · 评论 ${overview.commentsToday} · 赞 ${overview.likesToday}',
          ),
        ],
      ),
      AdminRecordGroup(
        title: '待处理队列',
        records: <AdminRecord>[
          _metric(
            'review-reports',
            '评课举报',
            overview.pendingReviewReports,
            'pending',
          ),
          _metric(
            'forum-flags',
            '论坛 flags',
            overview.pendingForumFlags,
            'open',
          ),
          _metric('dm-reports', '私信举报', overview.pendingDmReports, 'open'),
          _metric('media', '媒体上传', overview.pendingMediaUploads, 'pending'),
        ],
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _users(AdminActorContext actor) async {
    final Set<String> capabilities = actor.capabilities;
    if (!capabilities.contains(AdminCapabilities.searchUsers)) {
      return const <AdminRecordGroup>[
        AdminRecordGroup(
          title: '用户管理',
          description:
              '当前 capability 可以执行特定用户操作，但不包含 users.search；移动端不会越权读取用户目录。',
          records: <AdminRecord>[],
        ),
      ];
    }
    final AdminUserPage page = _required(
      await _api.adminUsersGet(limit: 30),
      '用户目录',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '用户目录',
        description: '仅显示服务端授权返回的管理视图，不包含公开响应之外的凭据。',
        hasMore: page.hasMore,
        records: page.items
            .map(
              (AdminUser user) => AdminRecord(
                id: user.id,
                title: '@${user.handle}',
                subtitle: '角色 ${user.role} · 状态 ${user.status}',
                evidence: <String>[
                  '信任等级 ${user.trustLevel}',
                  '创建 ${_unix(user.createdAt)}',
                  if (user.lastActiveAt != null)
                    '活跃 ${_unix(user.lastActiveAt!)}',
                ],
                actions: _userActions(user, actor),
              ),
            )
            .toList(growable: false),
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _moderation(Set<String> capabilities) async {
    final AdminForumFlagPage flags = _required(
      await _api.adminForumFlagsGet(limit: 20),
      '论坛 flags',
    );
    final ReviewPage reviews = _required(
      await _api.adminReviewsGet(status: 'all', limit: 20),
      '评课治理',
    );
    final ReportPage reports = _required(
      await _api.adminReportsGet(status: 'open', limit: 20),
      '评课举报',
    );
    final DmReportPage directMessages = _required(
      await _api.adminDmReportsGet(limit: 20),
      '私信举报',
    );
    final UploadPage uploads = _required(
      await _api.adminMediaUploadsGet(limit: 20),
      '媒体审核',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '论坛 flags',
        hasMore: flags.hasMore,
        records: flags.items
            .map(
              (AdminForumFlag flag) => AdminRecord(
                id: flag.id,
                title:
                    flag.targetTitle ?? '${flag.targetType} ${flag.targetId}',
                subtitle: flag.contentExcerpt ?? flag.note ?? '无文字证据摘要',
                evidence: <String>[
                  '原因 ${flag.reason}',
                  '权重 ${flag.weight}',
                  '状态 ${flag.status}',
                  if (flag.authorHandle != null) '作者 @${flag.authorHandle}',
                  '提交 ${_unix(flag.createdAt)}',
                ],
                actions: <AdminMutationAction>[
                  AdminMutationAction(
                    kind: AdminMutationKind.resolveForumFlag,
                    label: '处理论坛举报',
                    impact: '决定举报成立、驳回或忽略；决定和理由写入治理审计。',
                    requiredAnyCapability: const <String>{
                      AdminCapabilities.moderateContent,
                    },
                    targetId: flag.id,
                    isDestructive: true,
                    fields: const <AdminMutationField>[
                      AdminMutationField(
                        key: 'action',
                        label: '决定',
                        kind: AdminMutationFieldKind.choice,
                        initialValue: 'uphold',
                        options: <AdminMutationOption>[
                          AdminMutationOption('uphold', '举报成立'),
                          AdminMutationOption('reject', '驳回举报'),
                          AdminMutationOption('ignore', '忽略'),
                        ],
                      ),
                    ],
                  ),
                ],
              ),
            )
            .toList(growable: false),
      ),
      AdminRecordGroup(
        title: '评课治理',
        hasMore: reviews.hasMore,
        records: reviews.items
            .map(
              (Review review) => AdminRecord(
                id: review.id ?? 'unknown-review',
                title: review.authorHandle == null
                    ? '评课 ${review.id ?? '未知'}'
                    : '作者 @${review.authorHandle}',
                subtitle: review.comment ?? '无正文',
                evidence: <String>[
                  '状态 ${review.status ?? '未知'}',
                  if (review.rating != null) '${review.rating} 星',
                  if (review.courseId != null) '课程 ${review.courseId}',
                  if (review.createdAt != null)
                    '创建 ${_unix(review.createdAt!)}',
                ],
                actions: <AdminMutationAction>[
                  AdminMutationAction(
                    kind: AdminMutationKind.toggleReview,
                    label: review.status == ReviewStatusEnum.visible
                        ? '隐藏评课'
                        : '公开评课',
                    impact: '立即改变公共课程页面中的评课可见性。',
                    requiredAnyCapability: const <String>{
                      AdminCapabilities.moderateContent,
                    },
                    targetId: review.id,
                    isDestructive: review.status == ReviewStatusEnum.visible,
                  ),
                  AdminMutationAction(
                    kind: AdminMutationKind.deleteReview,
                    label: '移除评课',
                    impact: '执行可审计软删除，不直接清除历史记录。',
                    requiredAnyCapability: const <String>{
                      AdminCapabilities.moderateContent,
                    },
                    targetId: review.id,
                    isDestructive: true,
                  ),
                ],
              ),
            )
            .toList(growable: false),
      ),
      AdminRecordGroup(
        title: '评课举报',
        hasMore: reports.hasMore,
        records: reports.items
            .map(
              (Report report) => AdminRecord(
                id: report.id,
                title: report.courseId == null
                    ? '评课 ${report.reviewId}'
                    : '课程 ${report.courseId}',
                subtitle: report.reviewExcerpt ?? report.reason,
                evidence: <String>[
                  '原因 ${report.reason}',
                  '状态 ${report.status}',
                  if (report.reviewAuthorHandle != null)
                    '作者 @${report.reviewAuthorHandle}',
                  '提交 ${_unix(report.createdAt)}',
                ],
                actions: <AdminMutationAction>[
                  AdminMutationAction(
                    kind: AdminMutationKind.resolveReviewReport,
                    label: '处理评课举报',
                    impact: '提交明确举报状态，并把决定理由保留在治理审计中。',
                    requiredAnyCapability: const <String>{
                      AdminCapabilities.moderateContent,
                    },
                    targetId: report.id,
                    isDestructive: true,
                    fields: const <AdminMutationField>[
                      AdminMutationField(
                        key: 'action',
                        label: '决定',
                        kind: AdminMutationFieldKind.choice,
                        initialValue: 'uphold',
                        options: <AdminMutationOption>[
                          AdminMutationOption('uphold', '举报成立'),
                          AdminMutationOption('reject', '驳回举报'),
                          AdminMutationOption('ignore', '忽略'),
                        ],
                      ),
                    ],
                  ),
                ],
              ),
            )
            .toList(growable: false),
      ),
      AdminRecordGroup(
        title: '私信举报',
        hasMore: directMessages.hasMore,
        records: directMessages.items
            .map(
              (DmReport report) => AdminRecord(
                id: report.id,
                title: report.senderHandle == null
                    ? '消息 ${report.messageId}'
                    : '发送者 @${report.senderHandle}',
                subtitle: report.messageExcerpt ?? report.note ?? '无文字证据摘要',
                evidence: <String>[
                  '原因 ${report.reason}',
                  '状态 ${report.status}',
                  '会话 ${report.conversationId}',
                  '提交 ${_unix(report.createdAt)}',
                ],
                actions: <AdminMutationAction>[
                  AdminMutationAction(
                    kind: AdminMutationKind.resolveDirectMessageReport,
                    label: '处理私信举报',
                    impact: '决定举报成立或驳回；只使用服务端提供的受限证据摘要。',
                    requiredAnyCapability: const <String>{
                      AdminCapabilities.moderateContent,
                    },
                    targetId: report.id,
                    isDestructive: true,
                    fields: const <AdminMutationField>[
                      AdminMutationField(
                        key: 'action',
                        label: '决定',
                        kind: AdminMutationFieldKind.choice,
                        initialValue: 'uphold',
                        options: <AdminMutationOption>[
                          AdminMutationOption('uphold', '举报成立'),
                          AdminMutationOption('reject', '驳回举报'),
                        ],
                      ),
                    ],
                  ),
                ],
              ),
            )
            .toList(growable: false),
      ),
      _uploadGroup(uploads, title: '媒体审核', capabilities: capabilities),
    ];
  }

  Future<List<AdminRecordGroup>> _appeals() async {
    final AdminAppealPage page = _required(
      await _api.adminAppealsGet(limit: 30),
      '申诉队列',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '申诉队列',
        description: '版本字段用于 Web 决策时的并发冲突保护。',
        hasMore: page.hasMore,
        records: page.items
            .map(
              (AdminAppeal appeal) => AdminRecord(
                id: appeal.id,
                title: '${appeal.originalAction} · ${appeal.targetKind}',
                subtitle: appeal.submissionReason,
                evidence: <String>[
                  '状态 ${appeal.status}',
                  '处置 ${appeal.dispositionKind}',
                  '版本 ${appeal.version}',
                  '提交 ${_unix(appeal.submittedAt)}',
                  '可申诉至 ${_unix(appeal.appealableUntil)}',
                ],
                actions: <AdminMutationAction>[
                  if (appeal.status == AppealStatus.submitted)
                    AdminMutationAction(
                      kind: AdminMutationKind.startAppealReview,
                      label: '开始复核',
                      impact: '领取该申诉进行独立复核；使用当前版本防止重复接单。',
                      requiredAnyCapability: const <String>{
                        AdminCapabilities.reviewAppeals,
                      },
                      targetId: appeal.id,
                      expectedVersion: appeal.version,
                    ),
                  if (appeal.status == AppealStatus.inReview)
                    AdminMutationAction(
                      kind: AdminMutationKind.decideAppeal,
                      label: '提交申诉决定',
                      impact: '维持、撤销或修改原处置；版本冲突时必须刷新证据重新决定。',
                      requiredAnyCapability: const <String>{
                        AdminCapabilities.reviewAppeals,
                      },
                      targetId: appeal.id,
                      expectedVersion: appeal.version,
                      isDestructive: true,
                      fields: const <AdminMutationField>[
                        AdminMutationField(
                          key: 'outcome',
                          label: '结果',
                          kind: AdminMutationFieldKind.choice,
                          initialValue: 'upheld',
                          options: <AdminMutationOption>[
                            AdminMutationOption('upheld', '维持原决定'),
                            AdminMutationOption('overturned', '撤销原决定'),
                            AdminMutationOption('amended', '修改期限'),
                          ],
                        ),
                        AdminMutationField(
                          key: 'amendedEndsAt',
                          label: '修改后的截止 Unix 秒（仅修改期限）',
                          kind: AdminMutationFieldKind.integer,
                        ),
                      ],
                    ),
                ],
              ),
            )
            .toList(growable: false),
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _resources(Set<String> capabilities) async {
    final List<AdminRecordGroup> groups = <AdminRecordGroup>[];
    if (capabilities.contains(AdminCapabilities.manageCourses)) {
      final CoursePage courses = _required(
        await _api.adminCoursesGet(limit: 30),
        '课程资源',
      );
      groups.add(
        AdminRecordGroup(
          title: '课程资源',
          hasMore: courses.hasMore,
          records: courses.items
              .map(
                (Course course) => AdminRecord(
                  id: course.id ?? course.code ?? 'unknown-course',
                  title: course.name ?? '未命名课程',
                  subtitle: <String>[
                    if (course.code != null) course.code!,
                    if (course.teacherName != null) course.teacherName!,
                    if (course.department != null) course.department!,
                  ].join(' · '),
                  evidence: <String>[
                    '评课 ${course.reviewCount ?? 0}',
                    if (course.reviewAvg != null) '均分 ${course.reviewAvg}',
                    if (course.credit != null) '学分 ${course.credit}',
                  ],
                  actions: course.id == null
                      ? const <AdminMutationAction>[]
                      : <AdminMutationAction>[
                          AdminMutationAction(
                            kind: AdminMutationKind.updateCourse,
                            label: '编辑课程',
                            impact: '更新权威课程目录字段并影响公开课程页。',
                            requiredAnyCapability: const <String>{
                              AdminCapabilities.manageCourses,
                            },
                            targetId: course.id,
                            fields: <AdminMutationField>[
                              AdminMutationField(
                                key: 'code',
                                label: '课程代码',
                                initialValue: course.code ?? '',
                              ),
                              AdminMutationField(
                                key: 'name',
                                label: '课程名称',
                                initialValue: course.name ?? '',
                              ),
                              AdminMutationField(
                                key: 'credit',
                                label: '学分',
                                kind: AdminMutationFieldKind.decimal,
                                initialValue: course.credit?.toString() ?? '',
                              ),
                              AdminMutationField(
                                key: 'department',
                                label: '开课院系',
                                initialValue: course.department ?? '',
                              ),
                              AdminMutationField(
                                key: 'teacherName',
                                label: '教师',
                                initialValue: course.teacherName ?? '',
                              ),
                            ],
                          ),
                          AdminMutationAction(
                            kind: AdminMutationKind.deleteCourse,
                            label: '删除课程',
                            impact: '从课程目录移除记录；服务端保护已有引用与审计。',
                            requiredAnyCapability: const <String>{
                              AdminCapabilities.manageCourses,
                            },
                            targetId: course.id,
                            isDestructive: true,
                          ),
                        ],
                ),
              )
              .toList(growable: false),
        ),
      );
    }
    if (capabilities.contains(AdminCapabilities.manageCommunity)) {
      final List<Tag> tags = _required(await _api.adminForumTagsGet(), '论坛标签');
      final List<WatchedWord> watchedWords = _required(
        await _api.adminForumWatchedWordsGet(),
        '敏感词',
      );
      groups
        ..add(
          AdminRecordGroup(
            title: '论坛标签',
            records: tags
                .map(
                  (Tag tag) => AdminRecord(
                    id: tag.id ?? tag.slug ?? 'unknown-tag',
                    title: tag.name ?? tag.slug ?? '未命名标签',
                    subtitle: tag.description ?? '无描述',
                    evidence: <String>['主题 ${tag.threadCount ?? 0}'],
                    actions: tag.id == null
                        ? const <AdminMutationAction>[]
                        : <AdminMutationAction>[
                            AdminMutationAction(
                              kind: AdminMutationKind.updateTag,
                              label: '编辑标签',
                              impact: '立即影响论坛分类和检索。',
                              requiredAnyCapability: const <String>{
                                AdminCapabilities.manageCommunity,
                              },
                              targetId: tag.id,
                              fields: <AdminMutationField>[
                                AdminMutationField(
                                  key: 'slug',
                                  label: 'Slug',
                                  initialValue: tag.slug ?? '',
                                ),
                                AdminMutationField(
                                  key: 'name',
                                  label: '名称',
                                  initialValue: tag.name ?? '',
                                ),
                                AdminMutationField(
                                  key: 'description',
                                  label: '描述',
                                  kind: AdminMutationFieldKind.multiline,
                                  initialValue: tag.description ?? '',
                                ),
                              ],
                            ),
                            AdminMutationAction(
                              kind: AdminMutationKind.deleteTag,
                              label: '删除标签',
                              impact: '删除会解除标签与现有主题的关联。',
                              requiredAnyCapability: const <String>{
                                AdminCapabilities.manageCommunity,
                              },
                              targetId: tag.id,
                              isDestructive: true,
                            ),
                          ],
                  ),
                )
                .toList(growable: false),
          ),
        )
        ..add(
          AdminRecordGroup(
            title: '敏感词规则',
            records: watchedWords
                .map(
                  (WatchedWord word) => AdminRecord(
                    id: word.id ?? word.word ?? 'unknown-word',
                    title: word.word ?? '未命名规则',
                    subtitle: '动作 ${word.action ?? '未指定'}',
                    evidence: <String>[
                      if (word.createdAt != null)
                        '创建 ${_unix(word.createdAt!)}',
                    ],
                    actions: word.id == null
                        ? const <AdminMutationAction>[]
                        : <AdminMutationAction>[
                            AdminMutationAction(
                              kind: AdminMutationKind.deleteWatchedWord,
                              label: '删除关注词',
                              impact: '立即改变内容发布与审核规则。',
                              requiredAnyCapability: const <String>{
                                AdminCapabilities.manageCommunity,
                              },
                              targetId: word.id,
                              isDestructive: true,
                            ),
                          ],
                  ),
                )
                .toList(growable: false),
          ),
        );
    }
    if (capabilities.contains(AdminCapabilities.moderateContent)) {
      final UploadPage uploads = _required(
        await _api.adminMediaUploadsGet(limit: 20),
        '媒体资源',
      );
      groups.add(
        _uploadGroup(uploads, title: '媒体资源', capabilities: capabilities),
      );
    }
    return groups;
  }

  Future<List<AdminRecordGroup>> _activity() async {
    final ActivityPolicy activity = _required(
      await _api.adminActivityPolicyGet(),
      '活跃度策略',
    );
    final TrustLevelPolicy trust = _required(
      await _api.adminTrustPolicyGet(),
      '信任策略',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '活跃度策略',
        records: <AdminRecord>[
          AdminRecord(
            id: 'activity-${activity.version}',
            title: '版本 ${activity.version}',
            subtitle: activity.reason,
            evidence: <String>[
              '主题 ${activity.weights.thread}',
              '评论 ${activity.weights.comment}',
              '赞 ${activity.weights.like}',
              '签到 ${activity.weights.checkIn}',
              '变更 ${_unix(activity.createdAt)}',
            ],
            actions: <AdminMutationAction>[
              AdminMutationAction(
                kind: AdminMutationKind.updateActivityPolicy,
                label: '发布活跃度策略',
                impact: '发布新版本；历史原始计数不改写，但展示会按新权重重新解释。',
                requiredAnyCapability: const <String>{
                  AdminCapabilities.manageActivity,
                },
                expectedVersion: activity.version,
                fields: <AdminMutationField>[
                  AdminMutationField(
                    key: 'thread',
                    label: '主题权重',
                    kind: AdminMutationFieldKind.integer,
                    initialValue: activity.weights.thread.toString(),
                  ),
                  AdminMutationField(
                    key: 'comment',
                    label: '评论权重',
                    kind: AdminMutationFieldKind.integer,
                    initialValue: activity.weights.comment.toString(),
                  ),
                  AdminMutationField(
                    key: 'like',
                    label: '点赞权重',
                    kind: AdminMutationFieldKind.integer,
                    initialValue: activity.weights.like.toString(),
                  ),
                  AdminMutationField(
                    key: 'checkIn',
                    label: '签到权重',
                    kind: AdminMutationFieldKind.integer,
                    initialValue: activity.weights.checkIn.toString(),
                  ),
                ],
              ),
            ],
          ),
        ],
      ),
      AdminRecordGroup(
        title: '信任策略',
        records: <AdminRecord>[
          AdminRecord(
            id: 'trust-${trust.version}',
            title: '版本 ${trust.version}',
            subtitle: trust.reason,
            evidence: <String>[
              '积分策略 ${trust.scorePolicyVersion}',
              'L2 ${trust.thresholdLevel2} · L3 ${trust.thresholdLevel3}',
              'L4 ${trust.thresholdLevel4} · L5 ${trust.thresholdLevel5} · L6 ${trust.thresholdLevel6}',
              '赞日上限 ${trust.likeDailyCap}',
            ],
            actions: <AdminMutationAction>[
              AdminMutationAction(
                kind: AdminMutationKind.updateTrustPolicy,
                label: '发布信任策略',
                impact: '发布新版本并重新评估所有账号的自动信任等级。',
                requiredAnyCapability: const <String>{
                  AdminCapabilities.manageActivity,
                },
                expectedVersion: trust.version,
                fields: <AdminMutationField>[
                  for (final (String key, String label, int value) item
                      in <(String, String, int)>[
                        ('thresholdLevel2', 'Lv.2 阈值', trust.thresholdLevel2),
                        ('thresholdLevel3', 'Lv.3 阈值', trust.thresholdLevel3),
                        ('thresholdLevel4', 'Lv.4 阈值', trust.thresholdLevel4),
                        ('thresholdLevel5', 'Lv.5 阈值', trust.thresholdLevel5),
                        ('thresholdLevel6', 'Lv.6 阈值', trust.thresholdLevel6),
                        ('likeDailyCap', '每日点赞上限', trust.likeDailyCap),
                        (
                          'demotionCooldownDays',
                          '治理降级冷却天数',
                          trust.demotionCooldownDays,
                        ),
                      ])
                    AdminMutationField(
                      key: item.$1,
                      label: item.$2,
                      kind: AdminMutationFieldKind.integer,
                      initialValue: item.$3.toString(),
                    ),
                ],
              ),
            ],
          ),
        ],
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _announcements() async {
    final AnnouncementPage page = _required(
      await _api.adminAnnouncementsGet(limit: 30),
      '公告',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '公告',
        hasMore: page.hasMore,
        records: page.items
            .map(
              (Announcement item) => AdminRecord(
                id: item.id,
                title: item.title,
                subtitle: item.body ?? '无正文摘要',
                evidence: <String>[
                  '${item.status} · ${item.effectiveState}',
                  '受众 ${item.audience}',
                  '版本 ${item.version} / 修订 ${item.revision}',
                  '优先级 ${item.priority}',
                ],
                actions: _announcementActions(item),
              ),
            )
            .toList(growable: false),
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _promotions() async {
    final PromotionPage page = _required(
      await _api.adminPromotionsGet(limit: 30),
      '推广',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '推广',
        hasMore: page.hasMore,
        records: page.items
            .map(
              (Promotion item) => AdminRecord(
                id: item.id,
                title: item.title,
                subtitle: item.body ?? item.targetUrl,
                evidence: <String>[
                  '${item.status} · ${item.effectiveState}',
                  '位置 ${item.placement}',
                  '受众 ${item.audience}',
                  '版本 ${item.version} · 优先级 ${item.priority}',
                ],
                actions: _promotionActions(item),
              ),
            )
            .toList(growable: false),
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _achievements() async {
    final AchievementPage page = _required(
      await _api.adminAchievementsGet(limit: 30),
      '成就',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '成就定义',
        hasMore: page.hasMore,
        records: page.items
            .map(
              (Achievement item) => AdminRecord(
                id: item.id,
                title: item.name,
                subtitle: item.description ?? item.slug,
                evidence: <String>[
                  '状态 ${item.status}',
                  '版本 ${item.version}',
                  '贡献积分 ${item.mintAmount}',
                ],
                actions: <AdminMutationAction>[
                  AdminMutationAction(
                    kind: AdminMutationKind.updateAchievement,
                    label: '编辑成就定义',
                    impact: '更新定义版本；自动贡献积分只在规则首次命中时生效。',
                    requiredAnyCapability: const <String>{
                      AdminCapabilities.manageBadges,
                    },
                    targetId: item.id,
                    expectedVersion: item.version,
                    fields: <AdminMutationField>[
                      AdminMutationField(
                        key: 'name',
                        label: '名称',
                        initialValue: item.name,
                        isRequired: true,
                      ),
                      AdminMutationField(
                        key: 'description',
                        label: '描述',
                        kind: AdminMutationFieldKind.multiline,
                        initialValue: item.description ?? '',
                      ),
                      AdminMutationField(
                        key: 'icon',
                        label: '图标令牌',
                        kind: AdminMutationFieldKind.choice,
                        initialValue: item.icon.value,
                        isRequired: true,
                        options: const <AdminMutationOption>[
                          AdminMutationOption('award', '奖章'),
                          AdminMutationOption('book-open-check', '书本确认'),
                          AdminMutationOption('message-circle-heart', '社区贡献'),
                          AdminMutationOption('star', '星标'),
                        ],
                      ),
                      AdminMutationField(
                        key: 'status',
                        label: '状态',
                        kind: AdminMutationFieldKind.choice,
                        initialValue: item.status.value,
                        options: const <AdminMutationOption>[
                          AdminMutationOption('active', '启用'),
                          AdminMutationOption('retired', '停用'),
                        ],
                      ),
                      AdminMutationField(
                        key: 'mintAmount',
                        label: '自动规则积分',
                        kind: AdminMutationFieldKind.integer,
                        initialValue: item.mintAmount.toString(),
                      ),
                    ],
                  ),
                ],
              ),
            )
            .toList(growable: false),
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _verifications() async {
    final VerificationTypePage page = _required(
      await _api.adminVerificationsTypesGet(limit: 30),
      '认证类型',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '认证类型',
        hasMore: page.hasMore,
        records: page.items
            .map(
              (VerificationType item) => AdminRecord(
                id: item.id,
                title: item.label,
                subtitle: item.description ?? item.slug,
                evidence: <String>[
                  '类别 ${item.category}',
                  '徽标 ${item.badgeVariant}',
                  item.allowsPublicDisplay ? '允许公开展示' : '禁止公开展示',
                ],
              ),
            )
            .toList(growable: false),
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _creditIntegrity() async {
    final CreditReconciliationStats stats = _required(
      await _api.adminCreditReconciliationsStatsGet(),
      '对账统计',
    );
    final CreditReconciliationRunPage runs = _required(
      await _api.adminCreditReconciliationsGet(limit: 30),
      '对账记录',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '对账统计',
        records: <AdminRecord>[
          _metric(
            'total-runs',
            '总运行',
            stats.totalRuns,
            '失败 ${stats.failedRuns}',
          ),
          _metric(
            'drift-runs',
            '出现漂移',
            stats.runsWithDrift,
            '账本失败 ${stats.ledgerFailureRuns}',
          ),
        ],
      ),
      AdminRecordGroup(
        title: '对账运行',
        hasMore: runs.hasMore,
        records: runs.items
            .map(
              (CreditReconciliationRun run) => AdminRecord(
                id: run.id,
                title: '${run.status} · ${run.reason}',
                subtitle:
                    '检查 ${run.walletsChecked} 个钱包，漂移 ${run.driftedWallets}',
                evidence: <String>[
                  '余额漂移 ${run.balanceDriftedWallets}',
                  '序列漂移 ${run.sequenceDriftedWallets}',
                  '缺失 ${run.missingWallets}',
                  '绝对漂移 ${run.totalAbsoluteDrift}',
                  if (run.ledgerOk != null) '账本 ${run.ledgerOk! ? '通过' : '失败'}',
                ],
                actions: run.status == CreditReconciliationRunStatusEnum.failed
                    ? <AdminMutationAction>[
                        AdminMutationAction(
                          kind: AdminMutationKind.resumeCreditReconciliation,
                          label: '恢复对账',
                          impact: '从失败运行恢复只读扫描；不会自动修正余额。',
                          requiredAnyCapability: const <String>{
                            AdminCapabilities.manageCreditIntegrity,
                          },
                          targetId: run.id,
                        ),
                      ]
                    : const <AdminMutationAction>[],
              ),
            )
            .toList(growable: false),
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _audit() async {
    final AdminAuditEventPage page = _required(
      await _api.adminAuditEventsGet(limit: 40),
      '审计事件',
    );
    return <AdminRecordGroup>[
      AdminRecordGroup(
        title: '不可变审计事件',
        hasMore: page.hasMore,
        records: page.items
            .map(
              (AdminAuditEvent event) => AdminRecord(
                id: event.id,
                title: event.action,
                subtitle: '${event.targetType} · ${event.targetId}',
                evidence: <String>[
                  event.actorHandle == null
                      ? '操作者 ${event.actorKind}'
                      : '操作者 @${event.actorHandle}',
                  if (event.reason != null) '理由 ${event.reason}',
                  '发生 ${_unix(event.createdAt)}',
                ],
              ),
            )
            .toList(growable: false),
      ),
    ];
  }

  Future<List<AdminRecordGroup>> _system(Set<String> capabilities) async {
    final List<AdminRecordGroup> groups = <AdminRecordGroup>[];
    if (capabilities.contains(AdminCapabilities.managePlatform)) {
      final List<Setting> settings = _required(
        await _api.adminSettingsGet(),
        '平台设置',
      );
      groups.add(
        AdminRecordGroup(
          title: '平台设置',
          records: settings
              .map(
                (Setting setting) => AdminRecord(
                  id: setting.key ?? 'unknown-setting',
                  title: setting.key ?? '未命名设置',
                  subtitle: setting.value ?? '未设置',
                  actions: setting.key == null
                      ? const <AdminMutationAction>[]
                      : <AdminMutationAction>[
                          AdminMutationAction(
                            kind: AdminMutationKind.updateSetting,
                            label: '更新设置',
                            impact: '立即更新运行时平台设置；服务端验证键和值。',
                            requiredAnyCapability: const <String>{
                              AdminCapabilities.managePlatform,
                            },
                            targetId: setting.key,
                            fields: <AdminMutationField>[
                              AdminMutationField(
                                key: 'value',
                                label: '新值',
                                kind: AdminMutationFieldKind.multiline,
                                initialValue: setting.value ?? '',
                                isRequired: true,
                              ),
                            ],
                          ),
                        ],
                ),
              )
              .toList(growable: false),
        ),
      );
    }
    if (capabilities.contains(AdminCapabilities.runOperations)) {
      final NotificationOutboxEventPage outbox = _required(
        await _api.adminNotificationOutboxGet(limit: 20),
        '通知死信',
      );
      final AdminLifecycleJobPage lifecycle = _required(
        await _api.adminAccountLifecycleJobsGet(limit: 20),
        '账号生命周期作业',
      );
      final MediaDeletionJobPage deletionJobs = _required(
        await _api.adminMediaDeletionJobsGet(limit: 20),
        '媒体删除死信',
      );
      final MediaRetentionHoldPage retentionHolds = _required(
        await _api.adminMediaRetentionHoldsGet(limit: 20),
        '媒体保留',
      );
      final MediaReconciliationReport reconciliation = _required(
        await _api.adminMediaReconciliationGet(limit: 20),
        '媒体对账',
      );
      groups
        ..add(
          AdminRecordGroup(
            title: '通知出站死信',
            hasMore: outbox.hasMore,
            records: outbox.items
                .map(
                  (NotificationOutboxEvent event) => AdminRecord(
                    id: event.id,
                    title: event.eventType,
                    subtitle: '主题 ${event.topic} · 状态 ${event.state}',
                    evidence: <String>[
                      '尝试 ${event.attempts}/${event.maxAttempts}',
                      '人工重试 ${event.manualRetryCount}',
                      if (event.lastErrorCode != null)
                        '错误 ${event.lastErrorCode}',
                    ],
                    actions: <AdminMutationAction>[
                      AdminMutationAction(
                        kind: AdminMutationKind.retryNotificationOutbox,
                        label: '重试通知',
                        impact: '把死信事件重新排队；理由与重试元数据写入审计。',
                        requiredAnyCapability: const <String>{
                          AdminCapabilities.runOperations,
                        },
                        targetId: event.id,
                      ),
                    ],
                  ),
                )
                .toList(growable: false),
          ),
        )
        ..add(
          AdminRecordGroup(
            title: '账号生命周期作业',
            hasMore: lifecycle.hasMore,
            records: lifecycle.items
                .map(
                  (AdminLifecycleJob job) => AdminRecord(
                    id: job.id,
                    title: '${job.jobType} · @${job.accountHandle}',
                    subtitle: '账号状态 ${job.accountState} · 作业 ${job.status}',
                    evidence: <String>[
                      '尝试 ${job.attempts}',
                      if (job.lastErrorCode != null) '错误 ${job.lastErrorCode}',
                      '下次 ${_unix(job.nextAttemptAt)}',
                    ],
                    actions: job.status == AdminLifecycleJobStatusEnum.failed
                        ? <AdminMutationAction>[
                            AdminMutationAction(
                              kind: AdminMutationKind.requeueLifecycleJob,
                              label: '重新排队',
                              impact: '仅重新排队可恢复的失败作业；不修改账号业务目的。',
                              requiredAnyCapability: const <String>{
                                AdminCapabilities.runOperations,
                              },
                              targetId: job.id,
                            ),
                          ]
                        : const <AdminMutationAction>[],
                  ),
                )
                .toList(growable: false),
          ),
        )
        ..add(
          AdminRecordGroup(
            title: '媒体删除死信',
            hasMore: deletionJobs.hasMore,
            records: deletionJobs.items
                .map(
                  (MediaDeletionJob job) => AdminRecord(
                    id: job.id,
                    title: '上传 ${job.uploadId}',
                    subtitle: '${job.status} · ${job.reason}',
                    evidence: <String>[
                      '尝试 ${job.attemptCount}',
                      '来源 ${job.requestSource}',
                      if (job.lastErrorCode != null) '错误 ${job.lastErrorCode}',
                    ],
                    actions: job.status == MediaDeletionJobStatusEnum.deadLetter
                        ? <AdminMutationAction>[
                            AdminMutationAction(
                              kind: AdminMutationKind.retryMediaDeletion,
                              label: '重试删除',
                              impact: '仅重新排队隔离状态的删除死信。',
                              requiredAnyCapability: const <String>{
                                AdminCapabilities.runOperations,
                              },
                              targetId: job.id,
                              isDestructive: true,
                            ),
                          ]
                        : const <AdminMutationAction>[],
                  ),
                )
                .toList(growable: false),
          ),
        )
        ..add(
          AdminRecordGroup(
            title: '媒体保留',
            hasMore: retentionHolds.hasMore,
            records: retentionHolds.items
                .map(
                  (MediaRetentionHold hold) => AdminRecord(
                    id: hold.id,
                    title: '上传 ${hold.uploadId} · ${hold.holdKind}',
                    subtitle: hold.reason,
                    evidence: <String>[
                      '到期 ${_unix(hold.expiresAt)}',
                      hold.isExpired ? '已到期' : '生效中',
                    ],
                    actions: <AdminMutationAction>[
                      AdminMutationAction(
                        kind: AdminMutationKind.placeMediaRetentionHold,
                        label: '续期或替换保留',
                        impact: '使用 expectedHoldId 比较并交换，防止覆盖并发保留。',
                        requiredAnyCapability: const <String>{
                          AdminCapabilities.runOperations,
                        },
                        targetId: hold.uploadId,
                        fields: <AdminMutationField>[
                          AdminMutationField(
                            key: 'holdKind',
                            label: '保留类型',
                            kind: AdminMutationFieldKind.choice,
                            initialValue: hold.holdKind.value,
                            options: const <AdminMutationOption>[
                              AdminMutationOption('moderation', '治理'),
                              AdminMutationOption('security', '安全'),
                            ],
                          ),
                          AdminMutationField(
                            key: 'expiresAt',
                            label: '到期 Unix 秒',
                            kind: AdminMutationFieldKind.integer,
                            initialValue: hold.expiresAt.toString(),
                            isRequired: true,
                          ),
                          AdminMutationField(
                            key: 'expectedHoldId',
                            label: '已审阅保留 ID',
                            initialValue: hold.id,
                            isRequired: true,
                          ),
                        ],
                      ),
                      AdminMutationAction(
                        kind: AdminMutationKind.releaseMediaRetentionHold,
                        label: '释放保留',
                        impact: '使用 expectedHoldId 释放当前保留；不会覆盖并发续期。',
                        requiredAnyCapability: const <String>{
                          AdminCapabilities.runOperations,
                        },
                        targetId: hold.uploadId,
                        isDestructive: true,
                        fields: <AdminMutationField>[
                          AdminMutationField(
                            key: 'expectedHoldId',
                            label: '已审阅保留 ID',
                            initialValue: hold.id,
                            isRequired: true,
                          ),
                        ],
                      ),
                    ],
                  ),
                )
                .toList(growable: false),
          ),
        )
        ..add(
          AdminRecordGroup(
            title: '媒体对账',
            hasMore: reconciliation.nextCursor != null,
            records: reconciliation.items
                .map(
                  (MediaReconciliationFinding finding) => AdminRecord(
                    id: finding.assetId,
                    title: '资产 ${finding.assetId}',
                    subtitle: finding.issueCodes.join(' · '),
                    evidence: <String>[
                      'Provider inventory ${reconciliation.providerInventory}',
                      reconciliation.dryRun ? '只读检查' : '非 dry-run',
                    ],
                  ),
                )
                .toList(growable: false),
          ),
        );
    }
    return groups;
  }

  static List<AdminMutationAction> _announcementActions(Announcement item) {
    return <AdminMutationAction>[
      AdminMutationAction(
        kind: AdminMutationKind.updateAnnouncement,
        label: '编辑公告',
        impact: '更新公告并使用 expectedVersion 防止覆盖并发修订。',
        requiredAnyCapability: const <String>{
          AdminCapabilities.manageAnnouncements,
        },
        targetId: item.id,
        expectedVersion: item.version,
        fields: <AdminMutationField>[
          AdminMutationField(
            key: 'title',
            label: '标题',
            initialValue: item.title,
            isRequired: true,
          ),
          AdminMutationField(
            key: 'body',
            label: '正文',
            kind: AdminMutationFieldKind.multiline,
            initialValue: item.body ?? '',
          ),
          AdminMutationField(
            key: 'status',
            label: '状态',
            kind: AdminMutationFieldKind.choice,
            initialValue: item.status.value,
            isRequired: true,
            options: const <AdminMutationOption>[
              AdminMutationOption('draft', '草稿'),
              AdminMutationOption('scheduled', '排期'),
              AdminMutationOption('published', '发布'),
            ],
          ),
          AdminMutationField(
            key: 'presentation',
            label: '展示方式',
            kind: AdminMutationFieldKind.choice,
            initialValue: item.presentation.value,
            isRequired: true,
            options: const <AdminMutationOption>[
              AdminMutationOption('card', '卡片'),
              AdminMutationOption('banner', '横幅'),
            ],
          ),
          AdminMutationField(
            key: 'severity',
            label: '级别',
            kind: AdminMutationFieldKind.choice,
            initialValue: item.severity.value,
            isRequired: true,
            options: const <AdminMutationOption>[
              AdminMutationOption('info', '信息'),
              AdminMutationOption('success', '成功'),
              AdminMutationOption('warning', '警告'),
              AdminMutationOption('critical', '严重'),
            ],
          ),
          AdminMutationField(
            key: 'priority',
            label: '优先级',
            kind: AdminMutationFieldKind.integer,
            initialValue: item.priority.toString(),
          ),
          AdminMutationField(
            key: 'audience',
            label: '受众',
            kind: AdminMutationFieldKind.choice,
            initialValue: item.audience.value,
            isRequired: true,
            options: const <AdminMutationOption>[
              AdminMutationOption('all', '全部'),
              AdminMutationOption('authenticated', '已登录'),
              AdminMutationOption('staff', '工作人员'),
            ],
          ),
          AdminMutationField(
            key: 'requiresAck',
            label: '要求确认',
            kind: AdminMutationFieldKind.boolean,
            initialValue: item.requiresAck.toString(),
          ),
          AdminMutationField(
            key: 'startsAt',
            label: '开始 Unix 秒',
            kind: AdminMutationFieldKind.integer,
            initialValue: item.startsAt?.toString() ?? '',
          ),
          AdminMutationField(
            key: 'endsAt',
            label: '结束 Unix 秒',
            kind: AdminMutationFieldKind.integer,
            initialValue: item.endsAt?.toString() ?? '',
          ),
          const AdminMutationField(
            key: 'bumpRevision',
            label: '增加公开修订号',
            kind: AdminMutationFieldKind.boolean,
          ),
        ],
      ),
      AdminMutationAction(
        kind: AdminMutationKind.archiveAnnouncement,
        label: '归档公告',
        impact: '停止公告展示并保留修订历史。',
        requiredAnyCapability: const <String>{
          AdminCapabilities.manageAnnouncements,
        },
        targetId: item.id,
        expectedVersion: item.version,
        isDestructive: true,
      ),
    ];
  }

  static List<AdminMutationAction> _promotionActions(Promotion item) {
    return <AdminMutationAction>[
      AdminMutationAction(
        kind: AdminMutationKind.updatePromotion,
        label: '编辑推广',
        impact: '更新一方推广并使用 expectedVersion 防止覆盖并发排期。',
        requiredAnyCapability: const <String>{
          AdminCapabilities.managePromotions,
        },
        targetId: item.id,
        expectedVersion: item.version,
        fields: <AdminMutationField>[
          AdminMutationField(
            key: 'placement',
            label: '位置',
            kind: AdminMutationFieldKind.choice,
            initialValue: item.placement.value,
            isRequired: true,
            options: const <AdminMutationOption>[
              AdminMutationOption('home-left-primary', '首页左侧主位'),
              AdminMutationOption('home-left-secondary', '首页左侧次位'),
            ],
          ),
          AdminMutationField(
            key: 'title',
            label: '标题',
            initialValue: item.title,
            isRequired: true,
          ),
          AdminMutationField(
            key: 'body',
            label: '正文',
            kind: AdminMutationFieldKind.multiline,
            initialValue: item.body ?? '',
          ),
          AdminMutationField(
            key: 'ctaLabel',
            label: '按钮文字',
            initialValue: item.ctaLabel ?? '',
          ),
          AdminMutationField(
            key: 'targetUrl',
            label: '目标 URL',
            initialValue: item.targetUrl,
            isRequired: true,
          ),
          AdminMutationField(
            key: 'assetId',
            label: '媒体资产 ID',
            initialValue: item.assetId ?? '',
          ),
          AdminMutationField(
            key: 'status',
            label: '状态',
            kind: AdminMutationFieldKind.choice,
            initialValue: item.status.value,
            isRequired: true,
            options: const <AdminMutationOption>[
              AdminMutationOption('draft', '草稿'),
              AdminMutationOption('scheduled', '排期'),
              AdminMutationOption('published', '发布'),
              AdminMutationOption('paused', '暂停'),
            ],
          ),
          AdminMutationField(
            key: 'priority',
            label: '优先级',
            kind: AdminMutationFieldKind.integer,
            initialValue: item.priority.toString(),
          ),
          AdminMutationField(
            key: 'audience',
            label: '受众',
            kind: AdminMutationFieldKind.choice,
            initialValue: item.audience.value,
            isRequired: true,
            options: const <AdminMutationOption>[
              AdminMutationOption('all', '全部'),
              AdminMutationOption('authenticated', '已登录'),
              AdminMutationOption('staff', '工作人员'),
            ],
          ),
          AdminMutationField(
            key: 'startsAt',
            label: '开始 Unix 秒',
            kind: AdminMutationFieldKind.integer,
            initialValue: item.startsAt?.toString() ?? '',
          ),
          AdminMutationField(
            key: 'endsAt',
            label: '结束 Unix 秒',
            kind: AdminMutationFieldKind.integer,
            initialValue: item.endsAt?.toString() ?? '',
          ),
        ],
      ),
      AdminMutationAction(
        kind: AdminMutationKind.archivePromotion,
        label: '归档推广',
        impact: '停止推广展示并保留历史指标。',
        requiredAnyCapability: const <String>{
          AdminCapabilities.managePromotions,
        },
        targetId: item.id,
        expectedVersion: item.version,
        isDestructive: true,
      ),
    ];
  }

  static List<AdminMutationAction> _sectionActions(
    AdminSection section,
    AdminActorContext actor,
  ) {
    final Set<String> capabilities = actor.capabilities;
    final List<AdminMutationAction> actions = <AdminMutationAction>[];
    switch (section) {
      case AdminSection.overview || AdminSection.audit:
        break;
      case AdminSection.users:
        if (capabilities.contains(AdminCapabilities.inviteUsers)) {
          actions.add(
            const AdminMutationAction(
              kind: AdminMutationKind.inviteUser,
              label: '邀请校园用户',
              impact: '创建待验证邀请；不会设置密码或绕过校园邮箱所有权证明。',
              requiredAnyCapability: <String>{AdminCapabilities.inviteUsers},
              fields: <AdminMutationField>[
                AdminMutationField(
                  key: 'email',
                  label: '校园邮箱',
                  isRequired: true,
                ),
                AdminMutationField(
                  key: 'handle',
                  label: '公开 Handle',
                  isRequired: true,
                ),
              ],
            ),
          );
        }
      case AdminSection.moderation:
        actions.addAll(const <AdminMutationAction>[
          AdminMutationAction(
            kind: AdminMutationKind.moderateForumThread,
            label: '按 ID 治理主题',
            impact: '对指定主题执行显隐、关闭、归档、删除、移动或置顶操作。服务端再次校验目标与层级。',
            requiredAnyCapability: <String>{AdminCapabilities.moderateContent},
            isDestructive: true,
            fields: <AdminMutationField>[
              AdminMutationField(
                key: 'targetId',
                label: '主题 ID',
                isRequired: true,
              ),
              AdminMutationField(
                key: 'action',
                label: '动作',
                kind: AdminMutationFieldKind.choice,
                initialValue: 'hide',
                options: <AdminMutationOption>[
                  AdminMutationOption('hide', '隐藏'),
                  AdminMutationOption('unhide', '恢复显示'),
                  AdminMutationOption('close', '关闭回复'),
                  AdminMutationOption('reopen', '重新开放'),
                  AdminMutationOption('archive', '归档'),
                  AdminMutationOption('unarchive', '取消归档'),
                  AdminMutationOption('delete', '删除'),
                  AdminMutationOption('restore', '恢复删除'),
                  AdminMutationOption('pin', '置顶'),
                  AdminMutationOption('unpin', '取消置顶'),
                  AdminMutationOption('move', '移动板块'),
                ],
              ),
              AdminMutationField(key: 'boardId', label: '目标板块 ID（仅移动）'),
              AdminMutationField(
                key: 'globally',
                label: '全站置顶（仅置顶）',
                kind: AdminMutationFieldKind.boolean,
              ),
            ],
          ),
          AdminMutationAction(
            kind: AdminMutationKind.moderateForumComment,
            label: '按 ID 治理回复',
            impact: '对指定回复执行隐藏、恢复显示、删除或恢复删除。',
            requiredAnyCapability: <String>{AdminCapabilities.moderateContent},
            isDestructive: true,
            fields: <AdminMutationField>[
              AdminMutationField(
                key: 'targetId',
                label: '回复 ID',
                isRequired: true,
              ),
              AdminMutationField(
                key: 'action',
                label: '动作',
                kind: AdminMutationFieldKind.choice,
                initialValue: 'hide',
                options: <AdminMutationOption>[
                  AdminMutationOption('hide', '隐藏'),
                  AdminMutationOption('unhide', '恢复显示'),
                  AdminMutationOption('delete', '删除'),
                  AdminMutationOption('restore', '恢复删除'),
                ],
              ),
            ],
          ),
          AdminMutationAction(
            kind: AdminMutationKind.featureForumThread,
            label: '设置精选主题',
            impact: '改变主题的精选展示状态。',
            requiredAnyCapability: <String>{AdminCapabilities.moderateContent},
            fields: <AdminMutationField>[
              AdminMutationField(
                key: 'targetId',
                label: '主题 ID',
                isRequired: true,
              ),
              AdminMutationField(
                key: 'featured',
                label: '设为精选',
                kind: AdminMutationFieldKind.boolean,
                initialValue: 'true',
              ),
            ],
          ),
        ]);
      case AdminSection.appeals:
        break;
      case AdminSection.resources:
        if (capabilities.contains(AdminCapabilities.manageCourses)) {
          actions.add(
            const AdminMutationAction(
              kind: AdminMutationKind.createCourse,
              label: '新建课程',
              impact: '向权威课程目录新增一条记录。',
              requiredAnyCapability: <String>{AdminCapabilities.manageCourses},
              fields: <AdminMutationField>[
                AdminMutationField(
                  key: 'code',
                  label: '课程代码',
                  isRequired: true,
                ),
                AdminMutationField(
                  key: 'name',
                  label: '课程名称',
                  isRequired: true,
                ),
                AdminMutationField(
                  key: 'credit',
                  label: '学分',
                  kind: AdminMutationFieldKind.decimal,
                ),
                AdminMutationField(key: 'department', label: '开课院系'),
                AdminMutationField(key: 'teacherName', label: '教师'),
              ],
            ),
          );
        }
        if (capabilities.contains(AdminCapabilities.manageCommunity)) {
          actions.addAll(_communitySectionActions);
        }
      case AdminSection.activity:
        break;
      case AdminSection.announcements:
        actions.add(_createAnnouncementAction);
      case AdminSection.promotions:
        actions.add(_createPromotionAction);
      case AdminSection.achievements:
        actions.addAll(_achievementSectionActions);
      case AdminSection.verifications:
        actions.addAll(_verificationSectionActions);
      case AdminSection.creditIntegrity:
        actions.add(
          const AdminMutationAction(
            kind: AdminMutationKind.startCreditReconciliation,
            label: '启动完整对账',
            impact: '只读扫描账本与钱包派生缓存，不会修正余额；请求使用新的幂等键。',
            requiredAnyCapability: <String>{
              AdminCapabilities.manageCreditIntegrity,
            },
          ),
        );
      case AdminSection.system:
        if (capabilities.contains(AdminCapabilities.runOperations)) {
          actions.addAll(_operationsSectionActions);
        }
    }
    return actions;
  }

  static List<AdminMutationAction> _userActions(
    AdminUser user,
    AdminActorContext actor,
  ) {
    if (!actor.canManageTarget(accountId: user.id, role: user.role.value)) {
      return const <AdminMutationAction>[];
    }
    final List<AdminMutationAction> actions = <AdminMutationAction>[];
    if (actor.capabilities.contains(AdminCapabilities.changeRoles)) {
      actions.add(
        AdminMutationAction(
          kind: AdminMutationKind.changeUserRole,
          label: '变更角色',
          impact: '立即改变该账号的管理能力；服务端阻止越级、自操作和最后管理员变更。',
          requiredAnyCapability: const <String>{AdminCapabilities.changeRoles},
          targetId: user.id,
          targetAccountId: user.id,
          targetRole: user.role.value,
          isDestructive: true,
          fields: <AdminMutationField>[
            AdminMutationField(
              key: 'role',
              label: '新角色',
              kind: AdminMutationFieldKind.choice,
              initialValue: user.role == AdminUserRoleEnum.mod ? 'user' : 'mod',
              options: const <AdminMutationOption>[
                AdminMutationOption('user', '普通用户'),
                AdminMutationOption('mod', '版主'),
              ],
            ),
          ],
        ),
      );
    }
    if (actor.capabilities.contains(AdminCapabilities.silenceUsers) &&
        user.status == AdminUserStatusEnum.active) {
      actions.add(
        AdminMutationAction(
          kind: AdminMutationKind.silenceUser,
          label: '禁言',
          impact: '立即限制发言。截止时间必填，服务端还会按操作者层级限制最长时长。',
          requiredAnyCapability: const <String>{AdminCapabilities.silenceUsers},
          targetId: user.id,
          targetAccountId: user.id,
          targetRole: user.role.value,
          isDestructive: true,
          fields: const <AdminMutationField>[
            AdminMutationField(
              key: 'endsAt',
              label: '截止 Unix 秒',
              kind: AdminMutationFieldKind.integer,
              isRequired: true,
            ),
          ],
        ),
      );
    }
    if (actor.capabilities.contains(AdminCapabilities.suspendUsers)) {
      if (user.status == AdminUserStatusEnum.active) {
        actions.add(
          AdminMutationAction(
            kind: AdminMutationKind.suspendUser,
            label: '封禁',
            impact: '立即阻止账号访问；可选截止时间，留空代表永久封禁。',
            requiredAnyCapability: const <String>{
              AdminCapabilities.suspendUsers,
            },
            targetId: user.id,
            targetAccountId: user.id,
            targetRole: user.role.value,
            isDestructive: true,
            fields: const <AdminMutationField>[
              AdminMutationField(
                key: 'endsAt',
                label: '截止 Unix 秒（留空为永久）',
                kind: AdminMutationFieldKind.integer,
              ),
            ],
          ),
        );
      }
      actions
        ..add(
          AdminMutationAction(
            kind: AdminMutationKind.revokeUserSessions,
            label: '撤销全部会话',
            impact: '强制目标账号所有设备重新登录。',
            requiredAnyCapability: const <String>{
              AdminCapabilities.suspendUsers,
            },
            targetId: user.id,
            targetAccountId: user.id,
            targetRole: user.role.value,
            isDestructive: true,
          ),
        )
        ..add(
          AdminMutationAction(
            kind: AdminMutationKind.revokeSuspension,
            label: '撤销封禁记录',
            impact: '追加撤销事件，不覆盖原制裁记录；服务器要求近期认证。',
            requiredAnyCapability: const <String>{
              AdminCapabilities.suspendUsers,
            },
            targetId: user.id,
            targetAccountId: user.id,
            targetRole: user.role.value,
            fields: const <AdminMutationField>[
              AdminMutationField(
                key: 'sanctionId',
                label: '封禁记录 ID',
                isRequired: true,
              ),
            ],
          ),
        );
    }
    if (actor.capabilities.contains(AdminCapabilities.silenceUsers)) {
      actions.add(
        AdminMutationAction(
          kind: AdminMutationKind.revokeSilence,
          label: '撤销禁言记录',
          impact: '追加撤销事件，不覆盖原制裁记录。',
          requiredAnyCapability: const <String>{AdminCapabilities.silenceUsers},
          targetId: user.id,
          targetAccountId: user.id,
          targetRole: user.role.value,
          fields: const <AdminMutationField>[
            AdminMutationField(
              key: 'sanctionId',
              label: '禁言记录 ID',
              isRequired: true,
            ),
          ],
        ),
      );
    }
    if (actor.capabilities.contains(AdminCapabilities.manageActivity)) {
      actions.add(
        AdminMutationAction(
          kind: AdminMutationKind.adjustTrustLevel,
          label: '调整信任等级',
          impact: '手动设置会覆盖自动计算，直到清除覆盖。',
          requiredAnyCapability: const <String>{
            AdminCapabilities.manageActivity,
          },
          targetId: user.id,
          targetAccountId: user.id,
          targetRole: user.role.value,
          fields: <AdminMutationField>[
            AdminMutationField(
              key: 'trustLevel',
              label: '目标等级',
              kind: AdminMutationFieldKind.choice,
              initialValue: user.trustLevel.toString(),
              options: const <AdminMutationOption>[
                AdminMutationOption('1', 'Lv.1'),
                AdminMutationOption('2', 'Lv.2'),
                AdminMutationOption('3', 'Lv.3'),
                AdminMutationOption('4', 'Lv.4'),
                AdminMutationOption('5', 'Lv.5'),
                AdminMutationOption('6', 'Lv.6'),
              ],
            ),
            const AdminMutationField(
              key: 'clearOverride',
              label: '清除手动覆盖',
              kind: AdminMutationFieldKind.boolean,
            ),
          ],
        ),
      );
    }
    return actions;
  }

  static const List<AdminMutationAction>
  _communitySectionActions = <AdminMutationAction>[
    AdminMutationAction(
      kind: AdminMutationKind.createBoard,
      label: '新建板块',
      impact: '创建新的论坛板块并立即影响发帖分类。',
      requiredAnyCapability: <String>{AdminCapabilities.manageCommunity},
      fields: <AdminMutationField>[
        AdminMutationField(key: 'slug', label: 'Slug', isRequired: true),
        AdminMutationField(key: 'name', label: '名称', isRequired: true),
        AdminMutationField(
          key: 'description',
          label: '描述',
          kind: AdminMutationFieldKind.multiline,
        ),
        AdminMutationField(
          key: 'position',
          label: '排序',
          kind: AdminMutationFieldKind.integer,
        ),
        AdminMutationField(
          key: 'isLocked',
          label: '锁定板块',
          kind: AdminMutationFieldKind.boolean,
        ),
        AdminMutationField(
          key: 'minTrustToPost',
          label: '最低发帖信任等级',
          kind: AdminMutationFieldKind.integer,
        ),
        AdminMutationField(
          key: 'isQa',
          label: '问答板块',
          kind: AdminMutationFieldKind.boolean,
        ),
      ],
    ),
    AdminMutationAction(
      kind: AdminMutationKind.updateBoard,
      label: '按 ID 更新板块',
      impact: '更新指定板块；空字段保持现值。',
      requiredAnyCapability: <String>{AdminCapabilities.manageCommunity},
      fields: <AdminMutationField>[
        AdminMutationField(key: 'targetId', label: '板块 ID', isRequired: true),
        AdminMutationField(key: 'slug', label: 'Slug'),
        AdminMutationField(key: 'name', label: '名称'),
        AdminMutationField(
          key: 'description',
          label: '描述',
          kind: AdminMutationFieldKind.multiline,
        ),
        AdminMutationField(
          key: 'position',
          label: '排序',
          kind: AdminMutationFieldKind.integer,
        ),
        AdminMutationField(
          key: 'isLocked',
          label: '锁定状态',
          kind: AdminMutationFieldKind.choice,
          options: <AdminMutationOption>[
            AdminMutationOption('', '保持现值'),
            AdminMutationOption('true', '锁定'),
            AdminMutationOption('false', '解锁'),
          ],
        ),
        AdminMutationField(
          key: 'minTrustToPost',
          label: '最低发帖信任等级',
          kind: AdminMutationFieldKind.integer,
        ),
        AdminMutationField(
          key: 'isQa',
          label: '问答板块状态',
          kind: AdminMutationFieldKind.choice,
          options: <AdminMutationOption>[
            AdminMutationOption('', '保持现值'),
            AdminMutationOption('true', '是'),
            AdminMutationOption('false', '否'),
          ],
        ),
      ],
    ),
    AdminMutationAction(
      kind: AdminMutationKind.deleteBoard,
      label: '按 ID 删除板块',
      impact: '删除板块并解除现有主题的板块关联。',
      requiredAnyCapability: <String>{AdminCapabilities.manageCommunity},
      isDestructive: true,
      fields: <AdminMutationField>[
        AdminMutationField(key: 'targetId', label: '板块 ID', isRequired: true),
      ],
    ),
    AdminMutationAction(
      kind: AdminMutationKind.createTag,
      label: '新建标签',
      impact: '新增论坛标签。',
      requiredAnyCapability: <String>{AdminCapabilities.manageCommunity},
      fields: <AdminMutationField>[
        AdminMutationField(key: 'slug', label: 'Slug', isRequired: true),
        AdminMutationField(key: 'name', label: '名称', isRequired: true),
        AdminMutationField(
          key: 'description',
          label: '描述',
          kind: AdminMutationFieldKind.multiline,
        ),
      ],
    ),
    AdminMutationAction(
      kind: AdminMutationKind.createWatchedWord,
      label: '添加关注词',
      impact: '立即影响内容发布与审核队列；不得录入口令或无关个人信息。',
      requiredAnyCapability: <String>{AdminCapabilities.manageCommunity},
      fields: <AdminMutationField>[
        AdminMutationField(key: 'word', label: '关注词', isRequired: true),
        AdminMutationField(
          key: 'action',
          label: '动作',
          kind: AdminMutationFieldKind.choice,
          initialValue: 'queue',
          options: <AdminMutationOption>[
            AdminMutationOption('block', '阻止发布'),
            AdminMutationOption('censor', '脱敏'),
            AdminMutationOption('queue', '进入审核队列'),
          ],
        ),
      ],
    ),
  ];

  static const AdminMutationAction _createAnnouncementAction =
      AdminMutationAction(
        kind: AdminMutationKind.createAnnouncement,
        label: '新建公告',
        impact: '按状态和时间窗向指定受众发布一方公告。',
        requiredAnyCapability: <String>{AdminCapabilities.manageAnnouncements},
        fields: <AdminMutationField>[
          AdminMutationField(key: 'title', label: '标题', isRequired: true),
          AdminMutationField(
            key: 'body',
            label: '正文',
            kind: AdminMutationFieldKind.multiline,
          ),
          AdminMutationField(
            key: 'status',
            label: '状态',
            kind: AdminMutationFieldKind.choice,
            initialValue: 'draft',
            options: <AdminMutationOption>[
              AdminMutationOption('draft', '草稿'),
              AdminMutationOption('scheduled', '排期'),
              AdminMutationOption('published', '发布'),
            ],
          ),
          AdminMutationField(
            key: 'presentation',
            label: '展示方式',
            kind: AdminMutationFieldKind.choice,
            initialValue: 'card',
            options: <AdminMutationOption>[
              AdminMutationOption('card', '卡片'),
              AdminMutationOption('banner', '横幅'),
            ],
          ),
          AdminMutationField(
            key: 'severity',
            label: '级别',
            kind: AdminMutationFieldKind.choice,
            initialValue: 'info',
            options: <AdminMutationOption>[
              AdminMutationOption('info', '信息'),
              AdminMutationOption('success', '成功'),
              AdminMutationOption('warning', '警告'),
              AdminMutationOption('critical', '严重'),
            ],
          ),
          AdminMutationField(
            key: 'priority',
            label: '优先级',
            kind: AdminMutationFieldKind.integer,
            initialValue: '0',
          ),
          AdminMutationField(
            key: 'audience',
            label: '受众',
            kind: AdminMutationFieldKind.choice,
            initialValue: 'all',
            options: <AdminMutationOption>[
              AdminMutationOption('all', '全部'),
              AdminMutationOption('authenticated', '已登录'),
              AdminMutationOption('staff', '工作人员'),
            ],
          ),
          AdminMutationField(
            key: 'requiresAck',
            label: '要求确认',
            kind: AdminMutationFieldKind.boolean,
          ),
          AdminMutationField(
            key: 'startsAt',
            label: '开始 Unix 秒',
            kind: AdminMutationFieldKind.integer,
          ),
          AdminMutationField(
            key: 'endsAt',
            label: '结束 Unix 秒',
            kind: AdminMutationFieldKind.integer,
          ),
        ],
      );

  static const AdminMutationAction _createPromotionAction = AdminMutationAction(
    kind: AdminMutationKind.createPromotion,
    label: '新建推广',
    impact: '创建一方推广位；目标 URL 仍由服务端校验。',
    requiredAnyCapability: <String>{AdminCapabilities.managePromotions},
    fields: <AdminMutationField>[
      AdminMutationField(
        key: 'placement',
        label: '位置',
        kind: AdminMutationFieldKind.choice,
        initialValue: 'home-left-primary',
        options: <AdminMutationOption>[
          AdminMutationOption('home-left-primary', '首页左侧主位'),
          AdminMutationOption('home-left-secondary', '首页左侧次位'),
        ],
      ),
      AdminMutationField(key: 'title', label: '标题', isRequired: true),
      AdminMutationField(
        key: 'body',
        label: '正文',
        kind: AdminMutationFieldKind.multiline,
      ),
      AdminMutationField(key: 'ctaLabel', label: '按钮文字'),
      AdminMutationField(key: 'targetUrl', label: '目标 URL', isRequired: true),
      AdminMutationField(key: 'assetId', label: '媒体资产 ID'),
      AdminMutationField(
        key: 'status',
        label: '状态',
        kind: AdminMutationFieldKind.choice,
        initialValue: 'draft',
        options: <AdminMutationOption>[
          AdminMutationOption('draft', '草稿'),
          AdminMutationOption('scheduled', '排期'),
          AdminMutationOption('published', '发布'),
          AdminMutationOption('paused', '暂停'),
        ],
      ),
      AdminMutationField(
        key: 'priority',
        label: '优先级',
        kind: AdminMutationFieldKind.integer,
        initialValue: '0',
      ),
      AdminMutationField(
        key: 'audience',
        label: '受众',
        kind: AdminMutationFieldKind.choice,
        initialValue: 'all',
        options: <AdminMutationOption>[
          AdminMutationOption('all', '全部'),
          AdminMutationOption('authenticated', '已登录'),
          AdminMutationOption('staff', '工作人员'),
        ],
      ),
      AdminMutationField(
        key: 'startsAt',
        label: '开始 Unix 秒',
        kind: AdminMutationFieldKind.integer,
      ),
      AdminMutationField(
        key: 'endsAt',
        label: '结束 Unix 秒',
        kind: AdminMutationFieldKind.integer,
      ),
    ],
  );

  static const List<AdminMutationAction>
  _achievementSectionActions = <AdminMutationAction>[
    AdminMutationAction(
      kind: AdminMutationKind.createAchievement,
      label: '新建成就定义',
      impact: '创建贡献里程碑定义；人工授予不会发放积分。',
      requiredAnyCapability: <String>{AdminCapabilities.manageBadges},
      fields: <AdminMutationField>[
        AdminMutationField(key: 'slug', label: 'Slug', isRequired: true),
        AdminMutationField(key: 'name', label: '名称', isRequired: true),
        AdminMutationField(
          key: 'description',
          label: '描述',
          kind: AdminMutationFieldKind.multiline,
        ),
        AdminMutationField(
          key: 'icon',
          label: '图标令牌',
          kind: AdminMutationFieldKind.choice,
          initialValue: 'award',
          options: <AdminMutationOption>[
            AdminMutationOption('award', '奖章'),
            AdminMutationOption('book-open-check', '书本确认'),
            AdminMutationOption('message-circle-heart', '社区贡献'),
            AdminMutationOption('star', '星标'),
          ],
        ),
        AdminMutationField(
          key: 'mintAmount',
          label: '自动规则积分',
          kind: AdminMutationFieldKind.integer,
          initialValue: '0',
        ),
      ],
    ),
    AdminMutationAction(
      kind: AdminMutationKind.grantAchievement,
      label: '人工授予成就',
      impact: '仅授予荣誉展示，不会凭空发放积分。',
      requiredAnyCapability: <String>{AdminCapabilities.manageBadges},
      fields: <AdminMutationField>[
        AdminMutationField(key: 'accountId', label: '账号 ID', isRequired: true),
        AdminMutationField(
          key: 'achievementId',
          label: '成就 ID',
          isRequired: true,
        ),
      ],
    ),
    AdminMutationAction(
      kind: AdminMutationKind.revokeAchievement,
      label: '撤销用户成就',
      impact: '追加撤销事件；历史积分与账本不会被改写。',
      requiredAnyCapability: <String>{AdminCapabilities.manageBadges},
      isDestructive: true,
      fields: <AdminMutationField>[
        AdminMutationField(key: 'accountId', label: '账号 ID', isRequired: true),
        AdminMutationField(
          key: 'achievementId',
          label: '成就 ID',
          isRequired: true,
        ),
      ],
    ),
  ];

  static const List<AdminMutationAction>
  _verificationSectionActions = <AdminMutationAction>[
    AdminMutationAction(
      kind: AdminMutationKind.createVerificationType,
      label: '新建认证类型',
      impact: '创建可授予、可到期、可撤销的治理认证类型。',
      requiredAnyCapability: <String>{AdminCapabilities.manageVerifications},
      fields: <AdminMutationField>[
        AdminMutationField(key: 'slug', label: 'Slug', isRequired: true),
        AdminMutationField(
          key: 'category',
          label: '类别',
          kind: AdminMutationFieldKind.choice,
          initialValue: 'identity',
          options: <AdminMutationOption>[
            AdminMutationOption('identity', '身份'),
            AdminMutationOption('special', '特殊'),
          ],
        ),
        AdminMutationField(key: 'label', label: '名称', isRequired: true),
        AdminMutationField(
          key: 'description',
          label: '描述',
          kind: AdminMutationFieldKind.multiline,
        ),
        AdminMutationField(
          key: 'icon',
          label: '图标令牌',
          kind: AdminMutationFieldKind.choice,
          initialValue: 'badge-check',
          options: <AdminMutationOption>[
            AdminMutationOption('badge-check', '认证徽章'),
            AdminMutationOption('building-2', '机构'),
            AdminMutationOption('shield-check', '盾牌确认'),
            AdminMutationOption('sparkles', '特殊标识'),
          ],
        ),
        AdminMutationField(
          key: 'badgeVariant',
          label: '样式',
          kind: AdminMutationFieldKind.choice,
          initialValue: 'secondary',
          options: <AdminMutationOption>[
            AdminMutationOption('secondary', '次要'),
            AdminMutationOption('outline', '描边'),
            AdminMutationOption('default', '主要'),
          ],
        ),
        AdminMutationField(
          key: 'allowsPublicDisplay',
          label: '允许公开展示',
          kind: AdminMutationFieldKind.boolean,
        ),
      ],
    ),
    AdminMutationAction(
      kind: AdminMutationKind.grantVerification,
      label: '授予认证',
      impact: '基于已核验的私有证据授予治理凭证；证据引用不会公开。',
      requiredAnyCapability: <String>{AdminCapabilities.manageVerifications},
      fields: <AdminMutationField>[
        AdminMutationField(key: 'accountId', label: '账号 ID', isRequired: true),
        AdminMutationField(
          key: 'verificationTypeId',
          label: '认证类型 ID',
          isRequired: true,
        ),
        AdminMutationField(
          key: 'displayOnProfile',
          label: '公开展示',
          kind: AdminMutationFieldKind.boolean,
        ),
        AdminMutationField(
          key: 'expiresAt',
          label: '到期 Unix 秒',
          kind: AdminMutationFieldKind.integer,
        ),
        AdminMutationField(key: 'evidenceReference', label: '私有证据引用'),
      ],
    ),
    AdminMutationAction(
      kind: AdminMutationKind.revokeVerification,
      label: '撤销认证',
      impact: '撤销认证并保留历史记录。',
      requiredAnyCapability: <String>{AdminCapabilities.manageVerifications},
      isDestructive: true,
      fields: <AdminMutationField>[
        AdminMutationField(key: 'grantId', label: '认证授予 ID', isRequired: true),
      ],
    ),
  ];

  static const List<AdminMutationAction> _operationsSectionActions =
      <AdminMutationAction>[
        AdminMutationAction(
          kind: AdminMutationKind.triggerSelectionSync,
          label: '触发选课同步',
          impact: '排队执行一系统同步作业。',
          requiredAnyCapability: <String>{AdminCapabilities.runOperations},
        ),
        AdminMutationAction(
          kind: AdminMutationKind.reindexCourses,
          label: '重建课程索引',
          impact: '重建 Meilisearch 课程投影。',
          requiredAnyCapability: <String>{AdminCapabilities.runOperations},
        ),
        AdminMutationAction(
          kind: AdminMutationKind.reindexReviews,
          label: '重建评课索引',
          impact: '重建 Meilisearch 评课投影。',
          requiredAnyCapability: <String>{AdminCapabilities.runOperations},
        ),
        AdminMutationAction(
          kind: AdminMutationKind.reindexForum,
          label: '重建论坛索引',
          impact: '重建主题、用户、板块与标签搜索投影。',
          requiredAnyCapability: <String>{AdminCapabilities.runOperations},
        ),
      ];

  static T _required<T>(Response<T> response, String surface) {
    final T? data = response.data;
    if (data == null) {
      throw ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '$surface 返回了空响应',
        statusCode: response.statusCode,
      );
    }
    return data;
  }

  static AdminRecord _metric(
    String id,
    String label,
    int value,
    String context,
  ) {
    return AdminRecord(
      id: id,
      title: '$value',
      subtitle: label,
      evidence: <String>[context],
    );
  }

  static AdminRecordGroup _uploadGroup(
    UploadPage page, {
    required String title,
    required Set<String> capabilities,
  }) {
    return AdminRecordGroup(
      title: title,
      hasMore: page.hasMore,
      records: page.items
          .map(
            (Upload upload) => AdminRecord(
              id: upload.id,
              title: '${upload.kind} · ${upload.mime}',
              subtitle: '状态 ${upload.status} · 交付 ${upload.deliveryState}',
              evidence: <String>[
                '${upload.bytes} B',
                '审批证据 ${upload.approvalRequirement}',
                upload.isSelfReview ? '本人媒体，需独立复核' : '非本人媒体',
                if (upload.deliveryErrorCode != null)
                  '交付错误 ${upload.deliveryErrorCode}',
                if (upload.retentionHeld) '存在保留',
              ],
              actions: _uploadActions(upload, capabilities),
            ),
          )
          .toList(growable: false),
    );
  }

  static List<AdminMutationAction> _uploadActions(
    Upload upload,
    Set<String> capabilities,
  ) {
    final List<AdminMutationAction> actions = <AdminMutationAction>[];
    if (capabilities.contains(AdminCapabilities.moderateContent)) {
      if (upload.status == UploadStatusEnum.pending &&
          upload.kind == UploadKindEnum.image) {
        actions.add(
          AdminMutationAction(
            kind: AdminMutationKind.previewMedia,
            label: '安全预览',
            impact: '签发一次性、目的受限的预览令牌；不会暴露 OSS key 或持久 URL。',
            requiredAnyCapability: const <String>{
              AdminCapabilities.moderateContent,
            },
            targetId: upload.id,
            fields: upload.isSelfReview
                ? const <AdminMutationField>[
                    AdminMutationField(
                      key: 'selfReviewConfirmed',
                      label: '确认 ADMIN 本人媒体例外',
                      kind: AdminMutationFieldKind.boolean,
                      mustBeTrue: true,
                    ),
                  ]
                : const <AdminMutationField>[],
          ),
        );
      }
      if (upload.status == UploadStatusEnum.pending &&
          upload.approvalRequirement ==
              UploadApprovalRequirementEnum.satisfied) {
        actions.add(
          AdminMutationAction(
            kind: AdminMutationKind.approveMedia,
            label: '批准媒体',
            impact: '只在当前审核员已完成可信预览后批准，并排队生成安全交付版本。',
            requiredAnyCapability: const <String>{
              AdminCapabilities.moderateContent,
            },
            targetId: upload.id,
            isDestructive: true,
            fields: upload.isSelfReview
                ? const <AdminMutationField>[
                    AdminMutationField(
                      key: 'selfReviewConfirmed',
                      label: '确认 ADMIN 本人媒体例外',
                      kind: AdminMutationFieldKind.boolean,
                      mustBeTrue: true,
                    ),
                  ]
                : const <AdminMutationField>[],
          ),
        );
      }
      if (upload.status == UploadStatusEnum.pending ||
          upload.status == UploadStatusEnum.clean) {
        actions.add(
          AdminMutationAction(
            kind: AdminMutationKind.blockMedia,
            label: '隔离并删除',
            impact: '先阻止公开访问，再由持久作业清理交付版本和 Ingest 原件。',
            requiredAnyCapability: const <String>{
              AdminCapabilities.moderateContent,
            },
            targetId: upload.id,
            isDestructive: true,
            fields: upload.isSelfReview
                ? const <AdminMutationField>[
                    AdminMutationField(
                      key: 'selfReviewConfirmed',
                      label: '确认 ADMIN 本人媒体例外',
                      kind: AdminMutationFieldKind.boolean,
                      mustBeTrue: true,
                    ),
                  ]
                : const <AdminMutationField>[],
          ),
        );
      }
    }
    if (capabilities.contains(AdminCapabilities.runOperations)) {
      if (upload.status == UploadStatusEnum.clean &&
          upload.deliveryState == MediaDeliveryState.failed) {
        actions.add(
          AdminMutationAction(
            kind: AdminMutationKind.retryMediaProcessing,
            label: '重试安全版本处理',
            impact: '仅重新排队策略发布或交付处理失败的干净媒体。',
            requiredAnyCapability: const <String>{
              AdminCapabilities.runOperations,
            },
            targetId: upload.id,
          ),
        );
      }
      if (upload.status != UploadStatusEnum.blocked && !upload.retentionHeld) {
        actions.add(
          AdminMutationAction(
            kind: AdminMutationKind.placeMediaRetentionHold,
            label: '设置保留',
            impact: '暂停 Ingest 原件最终删除；必须限定类型和到期时间。',
            requiredAnyCapability: const <String>{
              AdminCapabilities.runOperations,
            },
            targetId: upload.id,
            fields: const <AdminMutationField>[
              AdminMutationField(
                key: 'holdKind',
                label: '保留类型',
                kind: AdminMutationFieldKind.choice,
                initialValue: 'moderation',
                options: <AdminMutationOption>[
                  AdminMutationOption('moderation', '治理'),
                  AdminMutationOption('security', '安全'),
                ],
              ),
              AdminMutationField(
                key: 'expiresAt',
                label: '到期 Unix 秒',
                kind: AdminMutationFieldKind.integer,
                isRequired: true,
              ),
            ],
          ),
        );
      }
    }
    return actions;
  }

  static String _unix(int seconds) {
    final DateTime time = DateTime.fromMillisecondsSinceEpoch(
      seconds * 1000,
      isUtc: true,
    ).toLocal();
    final String month = time.month.toString().padLeft(2, '0');
    final String day = time.day.toString().padLeft(2, '0');
    final String hour = time.hour.toString().padLeft(2, '0');
    final String minute = time.minute.toString().padLeft(2, '0');
    return '${time.year}-$month-$day $hour:$minute';
  }
}
