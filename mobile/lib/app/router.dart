import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../core/layout/adaptive_app_shell.dart';
import '../features/account/presentation/account_page.dart';
import '../features/admin/presentation/admin_center_page.dart';
import '../features/announcements/presentation/announcements_page.dart';
import '../features/appeals/presentation/appeals_page.dart';
import '../features/auth/data/session_manager.dart';
import '../features/auth/domain/session_state.dart';
import '../features/auth/presentation/login_page.dart';
import '../features/bookmarks/presentation/bookmarks_page.dart';
import '../features/courses/presentation/course_detail_page.dart';
import '../features/courses/presentation/courses_page.dart';
import '../features/forum/domain/forum_route_filters.dart';
import '../features/forum/presentation/forum_page.dart';
import '../features/forum/presentation/thread_detail_page.dart';
import '../features/home/presentation/home_page.dart';
import '../features/messages/data/messages_repository.dart';
import '../features/messages/presentation/messages_page.dart';
import '../features/notifications/presentation/notifications_page.dart';
import '../features/onboarding/presentation/onboarding_page.dart';
import '../features/profile/presentation/privacy_settings_page.dart';
import '../features/profile/presentation/profile_settings_page.dart';
import '../features/profile/presentation/public_profile_page.dart';
import '../features/profile/presentation/relationship_list_page.dart';
import '../features/schedule/presentation/schedule_page.dart';
import '../features/search/domain/search_models.dart';
import '../features/search/presentation/search_page.dart';
import '../features/settings/presentation/data_exports_page.dart';
import '../features/settings/presentation/lifecycle_page.dart';
import '../features/settings/presentation/notification_preferences_page.dart';
import '../features/settings/presentation/password_settings_page.dart';
import '../features/settings/presentation/recovery_page.dart';
import '../features/settings/presentation/sessions_page.dart';
import '../features/settings/presentation/settings_page.dart';
import '../features/wallet/presentation/wallet_page.dart';

abstract final class AppRoutes {
  static const home = '/';
  static const forum = '/forum';
  static const schedule = '/schedule';
  static const courses = '/courses';
  static const wallet = '/wallet';
  static const search = '/search';
  static const messages = '/messages';
  static const notifications = '/notifications';
  static const announcements = '/announcements';
  static const appeals = '/appeals';
  static const account = '/account';
  static const login = '/login';
  static const bookmarks = '/bookmarks';
  static const onboarding = '/onboarding';
  static const settings = '/settings';
  static const profileSettings = '/settings/profile';
  static const privacySettings = '/settings/privacy';
  static const notificationSettings = '/settings/notifications';
  static const sessions = '/settings/sessions';
  static const passwordSettings = '/settings/password';
  static const dataExports = '/settings/data-exports';
  static const lifecycle = '/settings/lifecycle';
  static const recovery = '/recover-account';
  static const admin = '/admin';
  static const rejectedLink = '/link-not-allowed';

  static String adminSection(String section) =>
      '$admin/${Uri.encodeComponent(section)}';

  static String thread(String id) =>
      '/forum/threads/${Uri.encodeComponent(id)}';

  static String course(String id) => '/courses/${Uri.encodeComponent(id)}';

  static String profile(String handle) =>
      '/profile/${Uri.encodeComponent(handle)}';

  static String profileFollowers(String handle) =>
      '${profile(handle)}/followers';

  static String profileFollowing(String handle) =>
      '${profile(handle)}/following';
}

String? focusedSessionRedirect(SessionState session, String location) {
  if (session.phase == SessionPhase.authenticated &&
      session.account?.onboardingRequired == true) {
    return location == AppRoutes.onboarding ? null : AppRoutes.onboarding;
  }
  if (location != AppRoutes.onboarding) {
    return null;
  }
  if (session.phase == SessionPhase.authenticated) {
    return AppRoutes.account;
  }
  if (session.phase == SessionPhase.restoring) {
    return null;
  }
  return AppRoutes.login;
}

String? appSessionRedirect(SessionState session, Uri location) {
  final Uri? trustedLocation = _trustedAppLocation(location);
  if (trustedLocation == null) {
    return AppRoutes.rejectedLink;
  }
  final String path = trustedLocation.path;
  final String routeLocation = Uri(
    path: path,
    queryParameters: trustedLocation.queryParameters.isEmpty
        ? null
        : trustedLocation.queryParameters,
  ).toString();
  if (session.phase == SessionPhase.authenticated &&
      session.account?.onboardingRequired == true) {
    if (path == AppRoutes.onboarding) {
      return null;
    }
    final String? returnLocation = path == AppRoutes.login
        ? safeLoginReturnLocation(trustedLocation.queryParameters['next'])
        : safeLoginReturnLocation(routeLocation);
    return _focusedLocation(AppRoutes.onboarding, returnLocation);
  }
  if (path == AppRoutes.onboarding) {
    if (session.phase == SessionPhase.authenticated) {
      return AppRoutes.account;
    }
    if (session.phase == SessionPhase.restoring) {
      return null;
    }
    return AppRoutes.login;
  }
  if (session.phase == SessionPhase.anonymous) {
    final String? returnLocation = safeAuthenticatedReturnLocation(
      routeLocation,
    );
    if (returnLocation != null) {
      return _focusedLocation(AppRoutes.login, returnLocation);
    }
  }
  return null;
}

Uri? _trustedAppLocation(Uri location) {
  if (location.hasFragment || location.userInfo.isNotEmpty) {
    return null;
  }
  final bool isRelative = !location.hasScheme && !location.hasAuthority;
  final bool isCustomScheme =
      location.scheme.toLowerCase() == 'yourtj' &&
      location.host.toLowerCase() == 'app' &&
      !location.hasPort;
  final bool isUniversalLink =
      location.scheme.toLowerCase() == 'https' &&
      location.host.toLowerCase() == 'yourtj.de' &&
      (!location.hasPort || location.port == 443);
  if (!isRelative && !isCustomScheme && !isUniversalLink) {
    return null;
  }
  if (location.path.isEmpty || !location.path.startsWith('/')) {
    return null;
  }
  return Uri(
    path: location.path,
    queryParameters: location.queryParameters.isEmpty
        ? null
        : location.queryParameters,
  );
}

String? safeAuthenticatedReturnLocation(String? rawLocation) {
  final Uri? parsed = _safeRelativeReturnUri(rawLocation);
  if (parsed == null) {
    return null;
  }
  final String path = parsed.path;
  if (path == AppRoutes.messages) {
    final Map<String, String> query = <String, String>{};
    final String? conversation = parsed.queryParameters['conversation'];
    if (conversation != null &&
        RegExp(r'^[A-Za-z0-9-]{1,128}$').hasMatch(conversation)) {
      query['conversation'] = conversation;
    }
    final String? view = parsed.queryParameters['view'];
    if (<String>{
      'inbox',
      'requests',
      'sent',
      'archived',
      'deleted',
    }.contains(view)) {
      query['view'] = view!;
    }
    final String? section = parsed.queryParameters['section'];
    if (section == 'announcements' ||
        section == 'notifications' ||
        section == 'direct-messages') {
      query['section'] = section!;
    }
    return Uri(
      path: path,
      queryParameters: query.isEmpty ? null : query,
    ).toString();
  }
  final bool isProtectedPath =
      <String>{
        AppRoutes.wallet,
        AppRoutes.notifications,
        AppRoutes.bookmarks,
        AppRoutes.account,
        AppRoutes.profileSettings,
        AppRoutes.privacySettings,
        AppRoutes.notificationSettings,
        AppRoutes.sessions,
        AppRoutes.passwordSettings,
        AppRoutes.dataExports,
        AppRoutes.lifecycle,
        AppRoutes.admin,
      }.contains(path) ||
      RegExp(r'^/admin/[A-Za-z0-9_-]{1,64}$').hasMatch(path);
  return isProtectedPath ? path : null;
}

String? safePublicInteractionReturnLocation(String? rawLocation) {
  final Uri? parsed = _safeRelativeReturnUri(rawLocation);
  if (parsed == null) {
    return null;
  }
  final String path = parsed.path;
  if (path == AppRoutes.home ||
      path == AppRoutes.courses ||
      path == AppRoutes.announcements) {
    return path;
  }
  if (path == AppRoutes.forum) {
    final Map<String, String> query = <String, String>{};
    final String? board = ForumRouteFilters.boardId(
      parsed.queryParameters['board'],
    );
    if (board != null) {
      query['board'] = board;
    }
    final String? tag = ForumRouteFilters.tagSlug(
      parsed.queryParameters['tag'],
    );
    if (tag != null) {
      query['tag'] = tag;
    }
    return Uri(
      path: path,
      queryParameters: query.isEmpty ? null : query,
    ).toString();
  }
  if (RegExp(r'^/courses/[A-Za-z0-9_-]{1,128}$').hasMatch(path)) {
    final String? reviewId = parsed.queryParameters['review'];
    if (reviewId != null &&
        RegExp(r'^[A-Za-z0-9_-]{1,128}$').hasMatch(reviewId)) {
      return Uri(
        path: path,
        queryParameters: <String, String>{'review': reviewId},
      ).toString();
    }
    return path;
  }
  if (RegExp(r'^/forum/threads/[A-Za-z0-9_-]{1,128}$').hasMatch(path) ||
      RegExp(
        r'^/profile/[a-z0-9._-]{3,30}(?:/(?:followers|following))?$',
      ).hasMatch(path)) {
    return path;
  }
  return null;
}

String? safeLoginReturnLocation(String? rawLocation) {
  return safePublicInteractionReturnLocation(rawLocation) ??
      safeAuthenticatedReturnLocation(rawLocation);
}

String publicInteractionLoginLocation(Uri currentLocation) {
  return _focusedLocation(
    AppRoutes.login,
    safePublicInteractionReturnLocation(currentLocation.toString()),
  );
}

Uri? _safeRelativeReturnUri(String? rawLocation) {
  if (rawLocation == null || rawLocation.length > 512) {
    return null;
  }
  final Uri? parsed = Uri.tryParse(rawLocation);
  if (parsed == null ||
      parsed.isAbsolute ||
      parsed.hasAuthority ||
      parsed.path.isEmpty ||
      !parsed.path.startsWith('/') ||
      parsed.path.startsWith('//') ||
      parsed.hasFragment ||
      parsed.userInfo.isNotEmpty) {
    return null;
  }
  return parsed;
}

String _focusedLocation(String path, String? returnLocation) {
  return Uri(
    path: path,
    queryParameters: returnLocation == null
        ? null
        : <String, String>{'next': returnLocation},
  ).toString();
}

GoRouter createAppRouter({
  String initialLocation = AppRoutes.home,
  SessionManager? session,
}) {
  final GlobalKey<NavigatorState> rootNavigatorKey = GlobalKey<NavigatorState>(
    debugLabel: 'root',
  );

  return GoRouter(
    navigatorKey: rootNavigatorKey,
    initialLocation: initialLocation,
    errorBuilder: (BuildContext context, GoRouterState state) {
      return Scaffold(
        appBar: AppBar(title: const Text('页面不存在')),
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                const Icon(Icons.link_off_rounded, size: 48),
                const SizedBox(height: 16),
                const Text('这个链接不在 YourTJ 的受支持页面中。'),
                const SizedBox(height: 16),
                FilledButton(
                  onPressed: () => context.go(AppRoutes.home),
                  child: const Text('返回首页'),
                ),
              ],
            ),
          ),
        ),
      );
    },
    refreshListenable: session?.routerRefresh,
    redirect: session == null
        ? null
        : (BuildContext context, GoRouterState state) {
            return appSessionRedirect(session.state, state.uri);
          },
    routes: <RouteBase>[
      StatefulShellRoute.indexedStack(
        builder:
            (
              BuildContext context,
              GoRouterState state,
              StatefulNavigationShell navigationShell,
            ) {
              return AdaptiveAppShell(navigationShell: navigationShell);
            },
        branches: <StatefulShellBranch>[
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: AppRoutes.home,
                builder: (BuildContext context, GoRouterState state) {
                  return const HomePage();
                },
              ),
            ],
          ),
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: AppRoutes.forum,
                builder: (BuildContext context, GoRouterState state) {
                  return ForumPage(
                    initialBoardId: ForumRouteFilters.boardId(
                      state.uri.queryParameters['board'],
                    ),
                    initialTag: ForumRouteFilters.tagSlug(
                      state.uri.queryParameters['tag'],
                    ),
                  );
                },
                routes: <RouteBase>[
                  GoRoute(
                    path: 'threads/:threadId',
                    builder: (BuildContext context, GoRouterState state) {
                      return ThreadDetailPage(
                        threadId: state.pathParameters['threadId']!,
                      );
                    },
                  ),
                ],
              ),
            ],
          ),
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: AppRoutes.schedule,
                builder: (BuildContext context, GoRouterState state) {
                  return const SchedulePage();
                },
              ),
            ],
          ),
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: AppRoutes.courses,
                builder: (BuildContext context, GoRouterState state) {
                  return const CoursesPage();
                },
                routes: <RouteBase>[
                  GoRoute(
                    path: ':courseId',
                    builder: (BuildContext context, GoRouterState state) {
                      return CourseDetailPage(
                        key: ValueKey<String>(state.uri.toString()),
                        courseId: state.pathParameters['courseId']!,
                        targetReviewId: state.uri.queryParameters['review'],
                      );
                    },
                  ),
                ],
              ),
            ],
          ),
          StatefulShellBranch(
            routes: <RouteBase>[
              GoRoute(
                path: AppRoutes.wallet,
                builder: (BuildContext context, GoRouterState state) {
                  return const WalletPage();
                },
              ),
            ],
          ),
        ],
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.search,
        builder: (BuildContext context, GoRouterState state) {
          return SearchPage(
            initialQuery: state.uri.queryParameters['q'] ?? '',
            initialScope: searchScopeFromWire(
              state.uri.queryParameters['type'],
            ),
          );
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.messages,
        builder: (BuildContext context, GoRouterState state) {
          final String? conversation =
              state.uri.queryParameters['conversation'];
          final String? rawView = state.uri.queryParameters['view'];
          final String? rawSection = state.uri.queryParameters['section'];
          final MessageCenterSection section = switch (rawSection) {
            'announcements' => MessageCenterSection.announcements,
            'notifications' => MessageCenterSection.notifications,
            'direct-messages' => MessageCenterSection.directMessages,
            _ when conversation != null || rawView != null =>
              MessageCenterSection.directMessages,
            _ => MessageCenterSection.notifications,
          };
          return MessagesPage(
            initialSection: section,
            initialConversationId: conversation,
            initialView: ConversationView.fromWire(rawView),
          );
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.notifications,
        builder: (BuildContext context, GoRouterState state) {
          return const NotificationsPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.announcements,
        builder: (BuildContext context, GoRouterState state) {
          return const AnnouncementsPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.appeals,
        builder: (BuildContext context, GoRouterState state) {
          return AppealsPage(
            initialEventId: state.uri.queryParameters['event'],
            initialAppealId: state.uri.queryParameters['appeal'],
          );
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.account,
        builder: (BuildContext context, GoRouterState state) {
          return const AccountPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.admin,
        builder: (BuildContext context, GoRouterState state) {
          return const AdminCenterPage();
        },
        routes: <RouteBase>[
          GoRoute(
            path: ':section',
            builder: (BuildContext context, GoRouterState state) {
              return AdminCenterPage(
                requestedSectionPath: state.pathParameters['section'],
              );
            },
          ),
        ],
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.login,
        builder: (BuildContext context, GoRouterState state) {
          return LoginPage(
            returnLocation: safeLoginReturnLocation(
              state.uri.queryParameters['next'],
            ),
          );
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.bookmarks,
        builder: (BuildContext context, GoRouterState state) {
          return const BookmarksPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.onboarding,
        builder: (BuildContext context, GoRouterState state) {
          return OnboardingPage(
            returnLocation: safeLoginReturnLocation(
              state.uri.queryParameters['next'],
            ),
          );
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.settings,
        builder: (BuildContext context, GoRouterState state) {
          return const SettingsPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.profileSettings,
        builder: (BuildContext context, GoRouterState state) {
          return const ProfileSettingsPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.privacySettings,
        builder: (BuildContext context, GoRouterState state) {
          return const PrivacySettingsPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.notificationSettings,
        builder: (BuildContext context, GoRouterState state) {
          return const NotificationPreferencesPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.sessions,
        builder: (BuildContext context, GoRouterState state) {
          return const SessionsPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.passwordSettings,
        builder: (BuildContext context, GoRouterState state) {
          return const PasswordSettingsPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.dataExports,
        builder: (BuildContext context, GoRouterState state) {
          return const DataExportsPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.lifecycle,
        builder: (BuildContext context, GoRouterState state) {
          return const LifecyclePage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: AppRoutes.recovery,
        builder: (BuildContext context, GoRouterState state) {
          return const RecoveryPage();
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: '/profile/:handle/followers',
        builder: (BuildContext context, GoRouterState state) {
          return RelationshipListPage(
            handle: state.pathParameters['handle']!,
            kind: RelationshipListKind.followers,
          );
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: '/profile/:handle/following',
        builder: (BuildContext context, GoRouterState state) {
          return RelationshipListPage(
            handle: state.pathParameters['handle']!,
            kind: RelationshipListKind.following,
          );
        },
      ),
      GoRoute(
        parentNavigatorKey: rootNavigatorKey,
        path: '/profile/:handle',
        builder: (BuildContext context, GoRouterState state) {
          return PublicProfilePage(handle: state.pathParameters['handle']!);
        },
      ),
    ],
  );
}

final GoRouter appRouter = createAppRouter();
