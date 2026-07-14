import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../app/app_services.dart';
import '../../auth/domain/session_state.dart';
import '../data/admin_mutation_executor.dart';
import '../data/admin_repository.dart';
import '../domain/admin_capabilities.dart';

final Provider<AdminRepository> adminRepositoryProvider =
    Provider<AdminRepository>((Ref ref) {
      return AdminRepository(
        GeneratedAdminReadDataSource(ref.watch(apiProvider).getAdminApi()),
      );
    });

final Provider<AdminMutationExecutor> adminMutationExecutorProvider =
    Provider<AdminMutationExecutor>((Ref ref) {
      return AdminMutationExecutor(ref.watch(apiProvider).getAdminApi());
    });

final adminSectionSnapshotProvider = FutureProvider.autoDispose
    .family<AdminSectionSnapshot, AdminSection>((
      Ref ref,
      AdminSection section,
    ) async {
      final SessionState session = await ref.watch(sessionStateProvider.future);
      final account = session.account;
      if (account == null) {
        throw const AdminAccessDenied();
      }
      return ref
          .watch(adminRepositoryProvider)
          .load(
            section,
            AdminActorContext(
              accountId: account.id,
              role: account.role.value,
              capabilities: account.capabilities.toSet(),
            ),
          );
    });
