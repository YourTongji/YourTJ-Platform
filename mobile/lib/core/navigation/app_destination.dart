import 'package:flutter/material.dart';

import '../../app/router.dart';
import '../l10n/app_strings.dart';

@immutable
class AppDestination {
  const AppDestination({
    required this.label,
    required this.path,
    required this.icon,
    required this.selectedIcon,
  });

  final String label;
  final String path;
  final IconData icon;
  final IconData selectedIcon;
}

const List<AppDestination> appDestinations = <AppDestination>[
  AppDestination(
    label: AppStrings.home,
    path: AppRoutes.home,
    icon: Icons.home_outlined,
    selectedIcon: Icons.home_rounded,
  ),
  AppDestination(
    label: AppStrings.forum,
    path: AppRoutes.forum,
    icon: Icons.forum_outlined,
    selectedIcon: Icons.forum_rounded,
  ),
  AppDestination(
    label: AppStrings.schedule,
    path: AppRoutes.schedule,
    icon: Icons.calendar_month_outlined,
    selectedIcon: Icons.calendar_month_rounded,
  ),
  AppDestination(
    label: AppStrings.courses,
    path: AppRoutes.courses,
    icon: Icons.menu_book_outlined,
    selectedIcon: Icons.menu_book_rounded,
  ),
  AppDestination(
    label: AppStrings.wallet,
    path: AppRoutes.wallet,
    icon: Icons.account_balance_wallet_outlined,
    selectedIcon: Icons.account_balance_wallet_rounded,
  ),
];
