import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../domain/search_models.dart';

abstract interface class FederatedSearchRepository {
  Future<SearchPageSlice> search({
    required String query,
    required SearchScope scope,
    required int limit,
    String? cursor,
    CancelToken? cancelToken,
  });
}

class GeneratedFederatedSearchRepository implements FederatedSearchRepository {
  const GeneratedFederatedSearchRepository(this._api);

  final SearchApi _api;

  @override
  Future<SearchPageSlice> search({
    required String query,
    required SearchScope scope,
    required int limit,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    try {
      final Response<SearchResult> response = await _api.searchGet(
        q: query,
        type: scope.wireValue,
        limit: limit,
        cursor: cursor,
        cancelToken: cancelToken,
      );
      final SearchResult? result = response.data;
      if (result == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '搜索响应不完整，请稍后重试',
        );
      }
      return SearchPageSlice(result: result);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }
}

final Provider<FederatedSearchRepository> federatedSearchRepositoryProvider =
    Provider<FederatedSearchRepository>((Ref ref) {
      return GeneratedFederatedSearchRepository(
        ref.watch(apiProvider).getSearchApi(),
      );
    });
