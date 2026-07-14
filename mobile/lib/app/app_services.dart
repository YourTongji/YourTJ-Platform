import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../core/config/app_environment.dart';
import '../core/network/session_interceptor.dart';
import '../core/storage/installation_store.dart';
import '../features/auth/data/secure_session_storage.dart';
import '../features/auth/data/session_manager.dart';
import '../features/auth/domain/session_state.dart';
import '../features/captcha/data/captcha_client.dart';
import '../features/media/data/media_uploader.dart';
import '../features/wallet/data/wallet_pending_mutation_store.dart';
import '../features/wallet/data/wallet_seed_store.dart';
import '../features/wallet/data/wallet_signer.dart';

class AppServices {
  AppServices._({
    required this.environment,
    required this.api,
    required this.captcha,
    required this.session,
    required this.mediaUploader,
    required this.walletPendingMutationStore,
    required this.walletSigner,
  });

  final AppEnvironment environment;
  final YourtjApi api;
  final CaptchaClient captcha;
  final SessionManager session;
  final MediaUploader mediaUploader;
  final WalletPendingMutationStore walletPendingMutationStore;
  final WalletSigner walletSigner;

  static Future<AppServices> create({
    AppEnvironment? environment,
    SecureSessionStorage? sessionStorage,
    InstallationStore? installationStore,
  }) async {
    final AppEnvironment resolvedEnvironment =
        environment ?? AppEnvironment.fromCompileTime();
    final BaseOptions publicOptions = BaseOptions(
      baseUrl: resolvedEnvironment.apiBaseUri.toString(),
      connectTimeout: const Duration(seconds: 10),
      receiveTimeout: const Duration(seconds: 20),
      sendTimeout: const Duration(seconds: 20),
      followRedirects: false,
      maxRedirects: 0,
      headers: const <String, Object>{'Accept': 'application/json'},
    );
    final Dio publicDio = Dio(publicOptions);
    final SessionManager session = SessionManager(
      AuthApi(publicDio),
      sessionStorage ??
          KeychainKeystoreSessionStorage(
            environmentNamespace: resolvedEnvironment.storageNamespace,
          ),
      installationStore ??
          createInstallationStore(
            environmentNamespace: resolvedEnvironment.storageNamespace,
          ),
    );
    final Dio apiDio = Dio(publicOptions.copyWith());
    apiDio.interceptors.add(
      SessionInterceptor(apiDio, resolvedEnvironment, session),
    );
    final YourtjApi api = YourtjApi(dio: apiDio, interceptors: const []);
    session.attachAuthenticatedApi(api.getAuthApi());
    final AppServices services = AppServices._(
      environment: resolvedEnvironment,
      api: api,
      captcha: CaptchaClient(environment: resolvedEnvironment),
      session: session,
      mediaUploader: MediaUploader(
        mediaApi: api.getMediaApi(),
        environment: resolvedEnvironment,
        ownerReader: () {
          final SessionState state = session.state;
          final String? accountId = state.account?.id;
          if (!state.isAuthenticated || accountId == null) {
            return null;
          }
          return MediaUploadOwner(
            accountId: accountId,
            generation: state.generation,
          );
        },
      ),
      walletPendingMutationStore: KeychainKeystoreWalletPendingMutationStore(
        environmentNamespace: resolvedEnvironment.storageNamespace,
      ),
      walletSigner: WalletSigner(
        KeychainKeystoreWalletSeedStore(
          environmentNamespace: resolvedEnvironment.storageNamespace,
        ),
      ),
    );
    await session.initialize();
    return services;
  }

  Future<void> dispose() => session.dispose();
}

final Provider<AppServices> appServicesProvider = Provider<AppServices>(
  (Ref ref) => throw StateError('AppServices must be overridden at startup'),
);

final Provider<YourtjApi> apiProvider = Provider<YourtjApi>(
  (Ref ref) => ref.watch(appServicesProvider).api,
);

final Provider<CaptchaClient> captchaClientProvider = Provider<CaptchaClient>(
  (Ref ref) => ref.watch(appServicesProvider).captcha,
);

final Provider<MediaUploader> mediaUploaderProvider = Provider<MediaUploader>(
  (Ref ref) => ref.watch(appServicesProvider).mediaUploader,
);

final Provider<WalletSigner> walletSignerProvider = Provider<WalletSigner>(
  (Ref ref) => ref.watch(appServicesProvider).walletSigner,
);

final Provider<WalletPendingMutationStore> walletPendingMutationStoreProvider =
    Provider<WalletPendingMutationStore>(
      (Ref ref) => ref.watch(appServicesProvider).walletPendingMutationStore,
    );

final Provider<SessionManager> sessionManagerProvider =
    Provider<SessionManager>(
      (Ref ref) => ref.watch(appServicesProvider).session,
    );

final StreamProvider<SessionState> sessionStateProvider =
    StreamProvider<SessionState>((Ref ref) async* {
      final SessionManager session = ref.watch(sessionManagerProvider);
      yield session.state;
      yield* session.changes;
    });
