import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../../auth/domain/session_state.dart';
import '../data/announcements_repository.dart';
import '../data/anonymous_announcement_seen_store.dart';
import 'announcement_dialog.dart';

class GlobalAnnouncementGate extends ConsumerStatefulWidget {
  const GlobalAnnouncementGate({
    required this.navigatorKey,
    required this.child,
    super.key,
  });

  final GlobalKey<NavigatorState> navigatorKey;
  final Widget child;

  @override
  ConsumerState<GlobalAnnouncementGate> createState() =>
      _GlobalAnnouncementGateState();
}

class _GlobalAnnouncementGateState extends ConsumerState<GlobalAnnouncementGate>
    with WidgetsBindingObserver {
  String? _viewerKey;
  SessionState? _viewer;
  int _requestGeneration = 0;
  bool _isDialogVisible = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    final SessionState? viewer = _viewer;
    if (state == AppLifecycleState.resumed &&
        viewer != null &&
        !_isDialogVisible) {
      _scheduleLoad(viewer, force: true);
    }
  }

  @override
  Widget build(BuildContext context) {
    final SessionState? viewer = ref.watch(sessionStateProvider).value;
    final bool isEligible =
        viewer != null &&
        (viewer.phase == SessionPhase.anonymous ||
            (viewer.phase == SessionPhase.authenticated &&
                viewer.account?.onboardingRequired != true));
    final String? nextViewerKey = isEligible ? _keyFor(viewer) : null;
    if (nextViewerKey != _viewerKey) {
      _viewerKey = nextViewerKey;
      _viewer = isEligible ? viewer : null;
      ++_requestGeneration;
      if (_isDialogVisible) {
        WidgetsBinding.instance.addPostFrameCallback((Duration _) {
          if (mounted && _isDialogVisible) {
            widget.navigatorKey.currentState?.maybePop();
          }
        });
      }
      if (isEligible) {
        _scheduleLoad(viewer);
      }
    }
    return widget.child;
  }

  String _keyFor(SessionState viewer) {
    return '${viewer.generation}:${viewer.account?.id ?? 'anonymous'}:'
        '${viewer.account?.onboardingRequired ?? false}';
  }

  void _scheduleLoad(SessionState viewer, {bool force = false}) {
    final int generation = force ? ++_requestGeneration : _requestGeneration;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      if (mounted && generation == _requestGeneration) {
        unawaited(_load(viewer, generation));
      }
    });
  }

  Future<void> _load(SessionState viewer, int generation) async {
    try {
      final AnnouncementsRepository repository = ref.read(
        announcementsRepositoryProvider,
      );
      final List<Announcement> queue;
      if (viewer.isAuthenticated) {
        queue = await repository.unread();
      } else {
        final List<Announcement> active = await repository.active();
        final Set<String> seen = await ref
            .read(anonymousAnnouncementSeenStoreProvider)
            .read();
        queue = active
            .where(
              (Announcement announcement) => !seen.contains(
                SharedPreferencesAnnouncementSeenStore.keyFor(announcement),
              ),
            )
            .toList(growable: false);
      }
      if (!_isCurrent(viewer, generation)) {
        return;
      }
      await _present(queue, viewer, generation);
    } on ApiFailure {
      // The gate never blocks the app when announcement delivery is unavailable.
    }
  }

  Future<void> _present(
    List<Announcement> queue,
    SessionState viewer,
    int generation,
  ) async {
    for (final Announcement announcement in queue) {
      if (!_isCurrent(viewer, generation)) {
        return;
      }
      final BuildContext? navigatorContext = widget.navigatorKey.currentContext;
      if (navigatorContext == null || !navigatorContext.mounted) {
        return;
      }
      final Completer<void> renderedOrClosed = Completer<void>();
      bool wasRendered = false;
      _isDialogVisible = true;
      final Future<AnnouncementReceiptInputActionEnum?> dialog =
          showAnnouncementDialog(
            context: navigatorContext,
            announcement: announcement,
            onPresented: () {
              wasRendered = true;
              if (!renderedOrClosed.isCompleted) {
                renderedOrClosed.complete();
              }
            },
          );
      unawaited(
        dialog.whenComplete(() {
          if (!renderedOrClosed.isCompleted) {
            renderedOrClosed.complete();
          }
        }),
      );
      await renderedOrClosed.future;
      if (wasRendered && _isCurrent(viewer, generation)) {
        await _recordSeen(announcement, viewer, generation);
      }
      final AnnouncementReceiptInputActionEnum? selected = await dialog;
      _isDialogVisible = false;
      if (!_isCurrent(viewer, generation)) {
        return;
      }
      if (viewer.isAuthenticated) {
        await _recordAction(
          announcement,
          selected ?? AnnouncementReceiptInputActionEnum.dismiss,
          viewer,
          generation,
        );
      }
    }
  }

  Future<void> _recordSeen(
    Announcement announcement,
    SessionState viewer,
    int generation,
  ) async {
    try {
      if (viewer.isAuthenticated) {
        await ref
            .read(announcementsRepositoryProvider)
            .record(
              announcement: announcement,
              action: AnnouncementReceiptInputActionEnum.seen,
            );
      } else {
        await ref
            .read(anonymousAnnouncementSeenStoreProvider)
            .remember(announcement);
      }
    } on ApiFailure {
      return;
    }
    if (!_isCurrent(viewer, generation)) {
      return;
    }
  }

  Future<void> _recordAction(
    Announcement announcement,
    AnnouncementReceiptInputActionEnum action,
    SessionState viewer,
    int generation,
  ) async {
    if (!_isCurrent(viewer, generation)) {
      return;
    }
    try {
      await ref
          .read(announcementsRepositoryProvider)
          .record(announcement: announcement, action: action);
    } on ApiFailure {
      return;
    }
  }

  bool _isCurrent(SessionState viewer, int generation) {
    return mounted &&
        generation == _requestGeneration &&
        _viewerKey == _keyFor(viewer);
  }
}
