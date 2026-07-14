import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/storage/installation_store.dart';
import 'package:yourtj_mobile/features/auth/data/secure_session_storage.dart';
import 'package:yourtj_mobile/features/auth/data/session_manager.dart';

typedef AdapterHandler =
    FutureOr<ResponseBody> Function(RequestOptions options);

class RecordedRequest {
  RecordedRequest.fromOptions(RequestOptions options)
    : uri = options.uri,
      method = options.method,
      headers = Map<String, dynamic>.from(options.headers),
      extra = Map<String, dynamic>.from(options.extra),
      data = options.data;

  final Uri uri;
  final String method;
  final Map<String, dynamic> headers;
  final Map<String, dynamic> extra;
  final Object? data;
}

class RecordingAdapter implements HttpClientAdapter {
  RecordingAdapter(this._handler);

  final AdapterHandler _handler;
  final List<RecordedRequest> requests = <RecordedRequest>[];
  bool isClosed = false;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<void>? cancelFuture,
  ) async {
    requests.add(RecordedRequest.fromOptions(options));
    return _handler(options);
  }

  @override
  void close({bool force = false}) {
    isClosed = true;
  }
}

class MemorySessionStorage implements SecureSessionStorage {
  MemorySessionStorage({
    String? activeAccountId,
    Map<String, String>? refreshTokens,
  }) : activeAccountId = activeAccountId,
       refreshTokens = refreshTokens ?? <String, String>{} {
    final String? refreshToken = this.refreshTokens[activeAccountId];
    if (activeAccountId != null && refreshToken != null) {
      _session = StoredSessionCredential(
        accountId: activeAccountId,
        refreshToken: refreshToken,
      );
    }
  }

  String? activeAccountId;
  final Map<String, String> refreshTokens;
  final List<String> clearedAccounts = <String>[];
  Object? readError;
  Object? replaceError;
  Object? clearError;
  StoredSessionCredential? _session;

  @override
  Future<StoredSessionCredential?> readSession() async {
    final Object? error = readError;
    if (error != null) {
      throw error;
    }
    return _session;
  }

  @override
  Future<void> replaceSession({
    required String accountId,
    required String refreshToken,
  }) async {
    final Object? error = replaceError;
    if (error != null) {
      throw error;
    }
    _session = StoredSessionCredential(
      accountId: accountId,
      refreshToken: refreshToken,
    );
    refreshTokens.clear();
    refreshTokens[accountId] = refreshToken;
    activeAccountId = accountId;
  }

  @override
  Future<void> clearSession(String accountId) async {
    clearedAccounts.add(accountId);
    final Object? error = clearError;
    if (error != null) {
      throw error;
    }
    if (activeAccountId == accountId) {
      activeAccountId = null;
      _session = null;
    }
    refreshTokens.remove(accountId);
  }
}

class FixedInstallationStore implements InstallationStore {
  FixedInstallationStore([
    this.installationId = 'b8ed936a-bf60-45c3-a2de-f3416cfa5559',
  ]);

  final String installationId;
  int readCount = 0;

  @override
  Future<String> readOrCreateId() async {
    readCount += 1;
    return installationId;
  }
}

class SessionHarness {
  SessionHarness({
    required AdapterHandler handler,
    MemorySessionStorage? storage,
    FixedInstallationStore? installationStore,
  }) : storage = storage ?? MemorySessionStorage(),
       installationStore = installationStore ?? FixedInstallationStore(),
       dio = Dio(
         BaseOptions(
           baseUrl: 'https://api.yourtj.de/api/v2',
           headers: const <String, Object>{'Accept': 'application/json'},
         ),
       ),
       adapter = RecordingAdapter(handler) {
    dio.httpClientAdapter = adapter;
    manager = SessionManager(
      AuthApi(dio),
      this.storage,
      this.installationStore,
    );
  }

  final Dio dio;
  final RecordingAdapter adapter;
  final MemorySessionStorage storage;
  final FixedInstallationStore installationStore;
  late final SessionManager manager;

  Future<void> dispose() async {
    await manager.dispose();
    dio.close(force: true);
  }
}

ResponseBody jsonResponse(Object? body, {int statusCode = 200}) {
  return ResponseBody.fromString(
    jsonEncode(body),
    statusCode,
    headers: <String, List<String>>{
      Headers.contentTypeHeader: <String>[Headers.jsonContentType],
    },
  );
}

Map<String, Object?> authTokensJson({
  required String accountId,
  required String accessToken,
  required String refreshToken,
  String? handle,
}) {
  return <String, Object?>{
    'accessToken': accessToken,
    'refreshToken': refreshToken,
    'account': <String, Object?>{
      'id': accountId,
      'handle': handle ?? accountId,
      'avatarUrl': null,
      'role': 'user',
      'capabilities': <String>[],
      'trustLevel': 1,
      'hasPassword': true,
      'onboardingRequired': false,
      'createdAt': 1,
    },
  };
}

Map<String, dynamic> requestJson(RecordedRequest request) {
  final Object? data = request.data;
  if (data is String) {
    return jsonDecode(data) as Map<String, dynamic>;
  }
  return Map<String, dynamic>.from(data! as Map);
}

Map<String, String> bearerSecurity() {
  return const <String, String>{
    'type': 'http',
    'scheme': 'bearer',
    'name': 'bearer',
  };
}
