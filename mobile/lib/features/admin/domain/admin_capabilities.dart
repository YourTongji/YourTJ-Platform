enum AdminSection {
  overview('overview'),
  users('users'),
  moderation('moderation'),
  appeals('appeals'),
  resources('resources'),
  activity('activity'),
  announcements('announcements'),
  promotions('promotions'),
  achievements('achievements'),
  verifications('verifications'),
  creditIntegrity('credit-integrity'),
  audit('audit'),
  system('system');

  const AdminSection(this.pathSegment);

  final String pathSegment;

  static AdminSection? fromPathSegment(String? value) {
    for (final AdminSection section in values) {
      if (section.pathSegment == value) {
        return section;
      }
    }
    return null;
  }
}

abstract final class AdminCapabilities {
  static const moderateContent = 'moderation.content';
  static const searchUsers = 'users.search';
  static const silenceUsers = 'users.silence';
  static const readAudit = 'audit.read';
  static const inviteUsers = 'users.invite';
  static const changeRoles = 'users.roles';
  static const suspendUsers = 'users.suspend';
  static const manageCommunity = 'community.manage';
  static const manageCourses = 'courses.manage';
  static const managePlatform = 'platform.settings';
  static const manageActivity = 'activity.policy';
  static const manageAnnouncements = 'announcements.manage';
  static const managePromotions = 'promotions.manage';
  static const manageBadges = 'badges.manage';
  static const manageVerifications = 'verifications.manage';
  static const runOperations = 'operations.jobs';
  static const manageCreditIntegrity = 'credit.integrity';
  static const reviewAppeals = 'appeals.review';
}

class AdminModule {
  const AdminModule({
    required this.section,
    required this.label,
    required this.description,
    required this.requiredAnyCapability,
    required this.mobileCoverage,
  });

  final AdminSection section;
  final String label;
  final String description;
  final Set<String> requiredAnyCapability;
  final String mobileCoverage;

  bool isVisibleTo(Set<String> capabilities) =>
      requiredAnyCapability.any(capabilities.contains);
}

const List<AdminModule> adminModules = <AdminModule>[
  AdminModule(
    section: AdminSection.overview,
    label: '概览',
    description: '队列、账号与社区状态',
    requiredAnyCapability: <String>{AdminCapabilities.searchUsers},
    mobileCoverage: '真实读取总览指标；不在移动端提供快捷写操作。',
  ),
  AdminModule(
    section: AdminSection.users,
    label: '用户',
    description: '账号、限制、会话与信任',
    requiredAnyCapability: <String>{
      AdminCapabilities.searchUsers,
      AdminCapabilities.inviteUsers,
      AdminCapabilities.changeRoles,
      AdminCapabilities.silenceUsers,
      AdminCapabilities.suspendUsers,
    },
    mobileCoverage: '读取用户目录，并按能力提供邀请、角色、制裁、会话撤销与信任调整。',
  ),
  AdminModule(
    section: AdminSection.moderation,
    label: '审核',
    description: '论坛、评课、私信与媒体队列',
    requiredAnyCapability: <String>{AdminCapabilities.moderateContent},
    mobileCoverage: '读取 flags、评课举报、私信举报与媒体证据，并提供对应治理操作。',
  ),
  AdminModule(
    section: AdminSection.appeals,
    label: '申诉',
    description: '独立复核与恢复',
    requiredAnyCapability: <String>{AdminCapabilities.reviewAppeals},
    mobileCoverage: '读取申诉队列和版本，并用 expectedVersion 保护接单与裁决。',
  ),
  AdminModule(
    section: AdminSection.resources,
    label: '内容资源',
    description: '课程、社区结构、媒体与索引',
    requiredAnyCapability: <String>{
      AdminCapabilities.moderateContent,
      AdminCapabilities.manageCourses,
      AdminCapabilities.manageCommunity,
    },
    mobileCoverage: '按能力读取并管理课程、社区结构与媒体；同步和索引作业受独立能力限制。',
  ),
  AdminModule(
    section: AdminSection.activity,
    label: '活跃度',
    description: '积分权重与信任策略',
    requiredAnyCapability: <String>{AdminCapabilities.manageActivity},
    mobileCoverage: '读取当前 activity/trust policy，并用版本条件更新策略。',
  ),
  AdminModule(
    section: AdminSection.announcements,
    label: '公告',
    description: '发布、排期与修订',
    requiredAnyCapability: <String>{AdminCapabilities.manageAnnouncements},
    mobileCoverage: '读取公告状态、受众和版本，并提供创建、发布、修订与归档。',
  ),
  AdminModule(
    section: AdminSection.promotions,
    label: '推广',
    description: '素材、排期与排序',
    requiredAnyCapability: <String>{AdminCapabilities.managePromotions},
    mobileCoverage: '读取推广状态、位置和版本，并提供创建、排期、排序与归档。',
  ),
  AdminModule(
    section: AdminSection.achievements,
    label: '成就',
    description: '定义、授予与撤销记录',
    requiredAnyCapability: <String>{AdminCapabilities.manageBadges},
    mobileCoverage: '读取并编辑成就定义，支持人工授予与可审计撤销。',
  ),
  AdminModule(
    section: AdminSection.verifications,
    label: '认证',
    description: '认证类型与用户标识',
    requiredAnyCapability: <String>{AdminCapabilities.manageVerifications},
    mobileCoverage: '读取认证类型，并提供创建类型、授予与可审计撤销。',
  ),
  AdminModule(
    section: AdminSection.creditIntegrity,
    label: '积分完整性',
    description: '账本与钱包对账',
    requiredAnyCapability: <String>{AdminCapabilities.manageCreditIntegrity},
    mobileCoverage: '读取对账统计和运行记录，支持幂等启动及失败运行恢复；不会修正余额。',
  ),
  AdminModule(
    section: AdminSection.audit,
    label: '审计',
    description: '不可变管理操作记录',
    requiredAnyCapability: <String>{AdminCapabilities.readAudit},
    mobileCoverage: '真实读取不可变审计事件；无写操作。',
  ),
  AdminModule(
    section: AdminSection.system,
    label: '平台',
    description: '设置、通知出站与生命周期作业',
    requiredAnyCapability: <String>{
      AdminCapabilities.managePlatform,
      AdminCapabilities.runOperations,
    },
    mobileCoverage: '按能力读取并管理设置、死信、媒体维护、账号生命周期与重建作业。',
  ),
];

List<AdminModule> adminModulesForCapabilities(
  Iterable<String> rawCapabilities,
) {
  final Set<String> capabilities = rawCapabilities.toSet();
  return adminModules
      .where((AdminModule module) => module.isVisibleTo(capabilities))
      .toList(growable: false);
}

AdminModule? adminModuleForSection(AdminSection section) {
  for (final AdminModule module in adminModules) {
    if (module.section == section) {
      return module;
    }
  }
  return null;
}
