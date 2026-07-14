import 'package:json_annotation/json_annotation.dart';
import 'package:yourtj_api/src/model/comment_draft_payload.dart';
import 'package:yourtj_api/src/model/thread_draft_payload.dart';

/// A cross-device forum draft decoded by its required `kind` discriminator.
sealed class ForumDraftPayload {
  const ForumDraftPayload();

  /// Wraps a thread draft for use in generated request and response models.
  const factory ForumDraftPayload.thread(ThreadDraftPayload payload) =
      ForumThreadDraftPayload;

  /// Wraps a comment draft for use in generated request and response models.
  const factory ForumDraftPayload.comment(CommentDraftPayload payload) =
      ForumCommentDraftPayload;

  /// Decodes the concrete draft shape selected by the wire discriminator.
  factory ForumDraftPayload.fromJson(Map<String, dynamic> json) {
    return switch (json['kind']) {
      'thread' => ForumDraftPayload.thread(ThreadDraftPayload.fromJson(json)),
      'comment' => ForumDraftPayload.comment(
        CommentDraftPayload.fromJson(json),
      ),
      _ => throw CheckedFromJsonException(
        json,
        'kind',
        'ForumDraftPayload',
        'Expected discriminator value `thread` or `comment`.',
      ),
    };
  }

  /// Encodes the concrete draft without adding a wrapper object on the wire.
  Map<String, dynamic> toJson();
}

/// The thread member of [ForumDraftPayload].
final class ForumThreadDraftPayload extends ForumDraftPayload {
  const ForumThreadDraftPayload(this.payload);

  /// The generated thread draft model.
  final ThreadDraftPayload payload;

  @override
  Map<String, dynamic> toJson() => payload.toJson();
}

/// The comment member of [ForumDraftPayload].
final class ForumCommentDraftPayload extends ForumDraftPayload {
  const ForumCommentDraftPayload(this.payload);

  /// The generated comment draft model.
  final CommentDraftPayload payload;

  @override
  Map<String, dynamic> toJson() => payload.toJson();
}
