import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../app/app_services.dart';
import '../../app/router.dart';
import '../../features/admin/domain/admin_capabilities.dart';
import '../../features/auth/domain/session_state.dart';
import '../../features/messages/domain/message_badge_counts.dart';
import '../l10n/app_strings.dart';
import '../navigation/app_destination.dart';
import '../navigation/app_route_visibility.dart';
import '../widgets/platform_avatar.dart';
import 'adaptive_breakpoints.dart';

class AdaptiveAppShell extends ConsumerWidget {
  const AdaptiveAppShell({required this.navigationShell, super.key});

  final StatefulNavigationShell navigationShell;

  void _selectDestination(int index) {
    navigationShell.goBranch(
      index,
      initialLocation: index == navigationShell.currentIndex,
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final double width = MediaQuery.sizeOf(context).width;
    final WindowSizeClass sizeClass = AdaptiveBreakpoints.classify(width);
    final SessionState? session = ref.watch(sessionStateProvider).value;

    return AppRouteVisibilityScope(
      isVisible: ModalRoute.isCurrentOf(context) ?? false,
      child: Scaffold(
        appBar: _AppHeader(
          sizeClass: sizeClass,
          messageCount: ref.watch(messageBadgeCountsProvider).value?.total ?? 0,
          isAuthenticated: session?.isAuthenticated == true,
          hasStaffCapabilities: adminModulesForCapabilities(
            session?.account?.capabilities ?? const <String>[],
          ).isNotEmpty,
          accountAvatar: session?.account == null
              ? const Icon(Icons.account_circle_outlined)
              : PlatformAvatar(
                  radius: 14,
                  // Account has a compatibility field until typed delivery lands.
                  // ignore: deprecated_member_use
                  compatibilityUrl: session!.account!.avatarUrl,
                  fallbackText: session.account!.handle,
                  semanticLabel: '${session.account!.handle} 的头像',
                  onRefresh: () => ref.invalidate(sessionStateProvider),
                ),
        ),
        body: switch (sizeClass) {
          WindowSizeClass.compact => navigationShell,
          WindowSizeClass.medium || WindowSizeClass.expanded => Row(
            children: <Widget>[
              _PrimaryNavigationRail(
                currentIndex: navigationShell.currentIndex,
                isExtended: sizeClass == WindowSizeClass.expanded,
                onDestinationSelected: _selectDestination,
              ),
              const VerticalDivider(width: 1),
              Expanded(child: navigationShell),
            ],
          ),
        },
        bottomNavigationBar: sizeClass == WindowSizeClass.compact
            ? _PrimaryNavigationBar(
                currentIndex: navigationShell.currentIndex,
                onDestinationSelected: _selectDestination,
              )
            : null,
      ),
    );
  }
}

class _AppHeader extends StatelessWidget implements PreferredSizeWidget {
  const _AppHeader({
    required this.sizeClass,
    required this.messageCount,
    required this.isAuthenticated,
    required this.hasStaffCapabilities,
    required this.accountAvatar,
  });

  final WindowSizeClass sizeClass;
  final int messageCount;
  final bool isAuthenticated;
  final bool hasStaffCapabilities;
  final Widget accountAvatar;

  @override
  Size get preferredSize => const Size.fromHeight(64);

  @override
  Widget build(BuildContext context) {
    final bool isExpanded = sizeClass == WindowSizeClass.expanded;

    return AppBar(
      titleSpacing: 16,
      title: const _Brand(),
      actions: <Widget>[
        if (isExpanded)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 12),
            child: OutlinedButton.icon(
              key: const Key('expanded-search-action'),
              onPressed: () => context.push(AppRoutes.search),
              icon: const Icon(Icons.search_rounded),
              label: const Text(AppStrings.searchHint),
            ),
          )
        else
          _HeaderAction(
            onPressed: () => context.push(AppRoutes.search),
            icon: const Icon(Icons.search_rounded),
            label: AppStrings.openSearch,
          ),
        _HeaderAction(
          onPressed: () => context.push(AppRoutes.messages),
          icon: Badge(
            isLabelVisible: messageCount > 0,
            label: Text(messageCount > 99 ? '99+' : '$messageCount'),
            child: const Icon(Icons.notifications_none_rounded),
          ),
          label: messageCount > 0
              ? '${AppStrings.openMessages}，$messageCount 条未读'
              : AppStrings.openMessages,
        ),
        Padding(
          padding: const EdgeInsets.only(right: 8),
          child: isAuthenticated
              ? _HeaderAccountMenu(
                  avatar: accountAvatar,
                  hasStaffCapabilities: hasStaffCapabilities,
                )
              : _HeaderAction(
                  onPressed: () => context.push(AppRoutes.account),
                  icon: accountAvatar,
                  label: AppStrings.openAccount,
                ),
        ),
      ],
    );
  }
}

class _HeaderAccountMenu extends StatelessWidget {
  const _HeaderAccountMenu({
    required this.avatar,
    required this.hasStaffCapabilities,
  });

  final Widget avatar;
  final bool hasStaffCapabilities;

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<String>(
      key: const Key('header-account-menu'),
      tooltip: AppStrings.openAccount,
      icon: avatar,
      onSelected: context.push,
      itemBuilder: (BuildContext context) => <PopupMenuEntry<String>>[
        _AccountMenuItem(
          route: AppRoutes.account,
          icon: Icons.person_outline_rounded,
          label: '账号',
        ),
        _AccountMenuItem(
          route: AppRoutes.announcements,
          icon: Icons.campaign_outlined,
          label: '公告',
        ),
        _AccountMenuItem(
          route: AppRoutes.bookmarks,
          icon: Icons.bookmark_outline_rounded,
          label: '收藏',
        ),
        _AccountMenuItem(
          route: AppRoutes.appeals,
          icon: Icons.gavel_outlined,
          label: '申诉',
        ),
        _AccountMenuItem(
          route: AppRoutes.settings,
          icon: Icons.settings_outlined,
          label: '设置',
        ),
        if (hasStaffCapabilities)
          _AccountMenuItem(
            route: AppRoutes.admin,
            icon: Icons.admin_panel_settings_outlined,
            label: '管理',
          ),
      ],
    );
  }
}

class _AccountMenuItem extends PopupMenuItem<String> {
  _AccountMenuItem({
    required String route,
    required IconData icon,
    required String label,
  }) : super(
         value: route,
         child: Row(
           children: <Widget>[Icon(icon), SizedBox(width: 12), Text(label)],
         ),
       );
}

class _HeaderAction extends StatelessWidget {
  const _HeaderAction({
    required this.onPressed,
    required this.icon,
    required this.label,
  });

  final VoidCallback onPressed;
  final Widget icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      container: true,
      label: label,
      onTap: onPressed,
      child: ExcludeSemantics(
        child: IconButton(onPressed: onPressed, icon: icon, tooltip: label),
      ),
    );
  }
}

class _Brand extends StatelessWidget {
  const _Brand();

  @override
  Widget build(BuildContext context) {
    return Semantics(
      header: true,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          ClipOval(
            child: Image.asset(
              'ios/Runner/Assets.xcassets/AppIcon.appiconset/'
              'Icon-App-1024x1024@1x.png',
              width: 32,
              height: 32,
              fit: BoxFit.cover,
              excludeFromSemantics: true,
            ),
          ),
          const SizedBox(width: 8),
          const Flexible(
            child: Text(
              AppStrings.appName,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

class _PrimaryNavigationBar extends StatelessWidget {
  const _PrimaryNavigationBar({
    required this.currentIndex,
    required this.onDestinationSelected,
  });

  final int currentIndex;
  final ValueChanged<int> onDestinationSelected;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      container: true,
      label: AppStrings.mainNavigation,
      child: NavigationBar(
        key: const Key('compact-navigation'),
        selectedIndex: currentIndex,
        onDestinationSelected: onDestinationSelected,
        destinations: appDestinations
            .map((AppDestination destination) {
              return NavigationDestination(
                icon: Icon(destination.icon),
                selectedIcon: Icon(destination.selectedIcon),
                label: destination.label,
              );
            })
            .toList(growable: false),
      ),
    );
  }
}

class _PrimaryNavigationRail extends StatelessWidget {
  const _PrimaryNavigationRail({
    required this.currentIndex,
    required this.isExtended,
    required this.onDestinationSelected,
  });

  final int currentIndex;
  final bool isExtended;
  final ValueChanged<int> onDestinationSelected;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      container: true,
      label: AppStrings.mainNavigation,
      child: NavigationRail(
        key: Key(
          isExtended ? 'expanded-navigation-rail' : 'medium-navigation-rail',
        ),
        selectedIndex: currentIndex,
        extended: isExtended,
        labelType: isExtended
            ? NavigationRailLabelType.none
            : NavigationRailLabelType.all,
        onDestinationSelected: onDestinationSelected,
        destinations: appDestinations
            .map((AppDestination destination) {
              return NavigationRailDestination(
                icon: Icon(destination.icon),
                selectedIcon: Icon(destination.selectedIcon),
                label: Text(destination.label),
              );
            })
            .toList(growable: false),
      ),
    );
  }
}
