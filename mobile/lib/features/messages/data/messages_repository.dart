import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

final Provider<MessagesRepository> messagesRepositoryProvider =
    Provider<MessagesRepository>((Ref ref) {
      return MessagesRepository(ref.watch(apiProvider).getForumApi());
    });

enum ConversationView {
  inbox('inbox', '收件箱'),
  requests('requests', '请求'),
  sent('sent', '已发送请求'),
  archived('archived', '已归档'),
  deleted('deleted', '已删除');

  const ConversationView(this.wireName, this.label);

  final String wireName;
  final String label;

  static ConversationView fromWire(String? value) => values.firstWhere(
    (ConversationView view) => view.wireName == value,
    orElse: () => inbox,
  );
}

class MessagesRepository {
  MessagesRepository(this._api);

  final ForumApi _api;

  Future<DmConversationPage> conversations({
    required ConversationView view,
    String? query,
    String? cursor,
  }) => _required(
    _api.forumDmConversationsGet(
      view: view.wireName,
      q: query != null && query.trim().length >= 2 ? query.trim() : null,
      cursor: cursor,
      limit: 20,
    ),
    '私信列表响应不完整，请重试',
  );

  Future<DmMessagePage> messages(String conversationId, {String? cursor}) =>
      _required(
        _api.forumDmConversationsIdMessagesGet(
          id: conversationId,
          cursor: cursor,
          limit: 30,
        ),
        '消息记录响应不完整，请重试',
      );

  Future<DmCounts> counts() =>
      _required(_api.forumDmUnreadCountGet(), '私信未读数响应不完整，请重试');

  Future<DmConversation> start({
    required String recipientHandle,
    required String requestMessage,
    required String idempotencyKey,
  }) => _required(
    _api.forumDmConversationsPost(
      dmConversationInput: DmConversationInput(
        recipientHandle: recipientHandle.trim(),
        requestMessage: requestMessage.trim().isEmpty
            ? null
            : requestMessage.trim(),
      ),
      idempotencyKey: idempotencyKey,
    ),
    '创建对话响应不完整，请刷新收件箱确认',
  );

  Future<DmMessage> send(
    String conversationId,
    String body, {
    String? clientMessageId,
  }) => _required(
    _api.forumDmConversationsIdMessagesPost(
      id: conversationId,
      dmMessageInput: DmMessageInput(
        body: body.trim(),
        clientMessageId: clientMessageId,
      ),
    ),
    '发送响应不完整，请刷新对话确认',
  );

  Future<void> markRead(String conversationId, String? messageId) => _empty(
    _api.forumDmConversationsIdReadPost(
      id: conversationId,
      dmReadInput: DmReadInput(lastReadMessageId: messageId),
    ),
  );

  Future<DmConversation> accept(String requestId) => _required(
    _api.forumDmRequestsIdAcceptPost(id: requestId),
    '接受请求响应不完整，请刷新收件箱确认',
  );

  Future<void> declineOrWithdraw(String requestId) =>
      _empty(_api.forumDmRequestsIdDelete(id: requestId));

  Future<void> archive(String conversationId) =>
      _empty(_api.forumDmConversationsIdArchivePut(id: conversationId));

  Future<void> unarchive(String conversationId) =>
      _empty(_api.forumDmConversationsIdArchiveDelete(id: conversationId));

  Future<void> delete(String conversationId) =>
      _empty(_api.forumDmConversationsIdDelete(id: conversationId));

  Future<void> recover(String conversationId) =>
      _empty(_api.forumDmConversationsIdRecoverPost(id: conversationId));

  Future<void> mute(String conversationId) =>
      _empty(_api.forumDmConversationsIdMutePut(id: conversationId));

  Future<void> unmute(String conversationId) =>
      _empty(_api.forumDmConversationsIdMuteDelete(id: conversationId));

  Future<void> reportMessage({
    required String messageId,
    required DmReportInputReasonEnum reason,
    String? note,
  }) => _empty(
    _api.forumDmMessagesIdReportPost(
      id: messageId,
      dmReportInput: DmReportInput(reason: reason, note: _cleanNote(note)),
    ),
  );

  Future<void> reportRequest({
    required String requestId,
    required DmReportInputReasonEnum reason,
    String? note,
  }) => _empty(
    _api.forumDmRequestsIdReportPost(
      id: requestId,
      dmReportInput: DmReportInput(reason: reason, note: _cleanNote(note)),
    ),
  );

  static String? _cleanNote(String? value) {
    final String? note = value?.trim();
    return note == null || note.isEmpty ? null : note;
  }

  Future<T> _required<T>(Future<Response<T>> request, String message) async {
    try {
      final T? value = (await request).data;
      if (value == null) {
        throw ApiFailure(kind: ApiFailureKind.unexpected, message: message);
      }
      return value;
    } on ApiFailure {
      rethrow;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '私信响应无法解析，请更新应用或稍后重试',
      );
    }
  }

  Future<void> _empty(Future<Response<void>> request) async {
    try {
      await request;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on Object {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '无法确认私信操作结果，请刷新服务器状态',
      );
    }
  }
}
