import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

enum SessionPhase {
  restoring,
  anonymous,
  authenticated,
  reconnectRequired,
  secureStorageUnavailable,
}

@immutable
class SessionState {
  const SessionState({
    required this.phase,
    required this.generation,
    this.account,
    this.message,
  });

  const SessionState.restoring({required int generation})
    : this(phase: SessionPhase.restoring, generation: generation);

  const SessionState.anonymous({required int generation})
    : this(phase: SessionPhase.anonymous, generation: generation);

  const SessionState.authenticated({
    required int generation,
    required Account account,
  }) : this(
         phase: SessionPhase.authenticated,
         generation: generation,
         account: account,
       );

  final SessionPhase phase;
  final int generation;
  final Account? account;
  final String? message;

  bool get isAuthenticated => phase == SessionPhase.authenticated;
}
