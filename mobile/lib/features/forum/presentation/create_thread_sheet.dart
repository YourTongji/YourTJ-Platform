import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../media/data/media_uploader.dart';
import '../../media/presentation/media_upload_button.dart';
import '../data/forum_repository.dart';
import 'forum_markdown_composer.dart';

Future<void> showCreateThreadSheet({
  required BuildContext context,
  required List<Board> boards,
}) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    builder: (BuildContext context) => CreateThreadSheet(boards: boards),
  );
}

class CreateThreadSheet extends ConsumerStatefulWidget {
  const CreateThreadSheet({required this.boards, super.key});

  final List<Board> boards;

  @override
  ConsumerState<CreateThreadSheet> createState() => _CreateThreadSheetState();
}

class _CreateThreadSheetState extends ConsumerState<CreateThreadSheet> {
  static const String _draftKey = 'thread:new';

  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _titleController = TextEditingController();
  final TextEditingController _bodyController = TextEditingController();
  final TextEditingController _tagsController = TextEditingController();
  final TextEditingController _pollQuestionController = TextEditingController();
  final TextEditingController _pollOptionsController = TextEditingController();
  Timer? _draftTimer;
  String? _boardId;
  int _draftVersion = 0;
  DraftOutput? _remoteConflict;
  bool _isLoadingDraft = true;
  bool _isSavingDraft = false;
  bool _isPublishing = false;
  bool _isPublished = false;
  String? _draftNotice;
  String? _error;

  ForumRepository get _repository => ref.read(forumRepositoryProvider);

  @override
  void initState() {
    super.initState();
    for (final TextEditingController controller in <TextEditingController>[
      _titleController,
      _bodyController,
      _tagsController,
      _pollQuestionController,
      _pollOptionsController,
    ]) {
      controller.addListener(_scheduleDraftSave);
    }
    unawaited(_loadDraft());
  }

  @override
  void dispose() {
    _draftTimer?.cancel();
    for (final TextEditingController controller in <TextEditingController>[
      _titleController,
      _bodyController,
      _tagsController,
      _pollQuestionController,
      _pollOptionsController,
    ]) {
      controller.dispose();
    }
    super.dispose();
  }

  bool get _hasContent =>
      _boardId != null ||
      _titleController.text.isNotEmpty ||
      _bodyController.text.isNotEmpty ||
      _tagsController.text.isNotEmpty ||
      _pollQuestionController.text.isNotEmpty ||
      _pollOptionsController.text.isNotEmpty;

  List<String> get _tags => _tagsController.text
      .split(RegExp(r'[,\s，、]+'))
      .map((String value) => value.trim())
      .where((String value) => value.isNotEmpty)
      .take(3)
      .toList();

  List<String> get _pollOptions => _pollOptionsController.text
      .split(RegExp(r'\n+'))
      .map((String value) => value.trim())
      .where((String value) => value.isNotEmpty)
      .take(20)
      .toList();

  Set<String> get _attachmentAssetIds =>
      RegExp(r'!\[[^\]]*\]\(yourtj-asset:([1-9][0-9]*)\)')
          .allMatches(_bodyController.text)
          .map((RegExpMatch match) => match.group(1)!)
          .toSet();

  ThreadDraftPayload get _draftPayload => ThreadDraftPayload(
    kind: ThreadDraftPayloadKindEnum.thread,
    boardId: _boardId,
    title: _titleController.text,
    body: _bodyController.text,
    contentFormat: ContentFormat.markdownV1,
    tags: _tags,
    pollQuestion: _pollQuestionController.text,
    pollOptions: _pollOptions,
    attachmentAssetIds: _attachmentAssetIds,
  );

  Board? get _selectedBoard {
    for (final Board board in widget.boards) {
      if (board.id == _boardId) {
        return board;
      }
    }
    return null;
  }

  Future<void> _loadDraft() async {
    try {
      final DraftOutput? draft = await _repository.draft(_draftKey);
      if (!mounted || draft == null) {
        return;
      }
      _draftVersion = draft.version;
      final ForumDraftPayload payload = draft.payload;
      if (payload is ForumThreadDraftPayload) {
        _restore(payload.payload);
        setState(() => _draftNotice = '已恢复跨设备草稿');
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _draftNotice = '云端草稿暂不可用：${failure.message}');
      }
    } finally {
      if (mounted) {
        setState(() => _isLoadingDraft = false);
      }
    }
  }

  void _restore(ThreadDraftPayload payload) {
    _boardId = payload.boardId;
    _titleController.text = payload.title;
    _bodyController.text = payload.body;
    _tagsController.text = payload.tags.join(' ');
    _pollQuestionController.text = payload.pollQuestion;
    _pollOptionsController.text = payload.pollOptions.join('\n');
  }

  void _scheduleDraftSave() {
    if (_isLoadingDraft || _isPublishing || _isPublished) {
      return;
    }
    _draftTimer?.cancel();
    _draftTimer = Timer(const Duration(milliseconds: 900), () {
      unawaited(_saveDraft());
    });
  }

  Future<void> _saveDraft({int? expectedVersion}) async {
    if (_isSavingDraft || _isPublished || !_hasContent) {
      return;
    }
    if (mounted) {
      setState(() {
        _isSavingDraft = true;
        _draftNotice = null;
      });
    }
    try {
      final DraftOutput saved = await _repository.saveDraft(
        DraftSaveInput(
          draftKey: _draftKey,
          expectedVersion: expectedVersion ?? _draftVersion,
          payload: ForumDraftPayload.thread(_draftPayload),
        ),
      );
      _draftVersion = saved.version;
      _remoteConflict = null;
      if (mounted) {
        setState(() => _draftNotice = '草稿已同步');
      }
    } on ApiFailure catch (failure) {
      if (failure.kind == ApiFailureKind.conflict) {
        await _loadConflict();
      } else if (mounted) {
        setState(() => _draftNotice = '草稿未同步：${failure.message}');
      }
    } finally {
      if (mounted) {
        setState(() => _isSavingDraft = false);
      }
    }
  }

  Future<void> _loadConflict() async {
    try {
      final DraftOutput? latest = await _repository.draft(_draftKey);
      if (mounted) {
        setState(() {
          _remoteConflict = latest;
          _draftNotice = latest == null
              ? '远端草稿已删除；本地内容仍保留'
              : '另一台设备修改了草稿；请选择保留哪一版';
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _draftNotice = '草稿冲突且无法读取远端：${failure.message}');
      }
    }
  }

  void _useRemote() {
    final DraftOutput? remote = _remoteConflict;
    if (remote == null) {
      return;
    }
    final ForumDraftPayload payload = remote.payload;
    if (payload is ForumThreadDraftPayload) {
      _restore(payload.payload);
      setState(() {
        _draftVersion = remote.version;
        _remoteConflict = null;
        _draftNotice = '已使用云端版本';
      });
    }
  }

  Future<void> _keepLocal() async {
    final DraftOutput? remote = _remoteConflict;
    if (remote == null) {
      return;
    }
    _draftVersion = remote.version;
    await _saveDraft(expectedVersion: remote.version);
  }

  Future<void> _publish() async {
    if (_isPublishing || !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    final Board? board = _selectedBoard;
    if (board == null || !board.canPost) {
      setState(() => _error = '你当前没有在此板块发帖的权限');
      return;
    }
    final List<String> options = _pollOptions;
    final String pollQuestion = _pollQuestionController.text.trim();
    if (pollQuestion.isNotEmpty && options.length < 2) {
      setState(() => _error = '投票至少需要两个选项');
      return;
    }
    setState(() {
      _isPublishing = true;
      _error = null;
    });
    try {
      final ThreadDetail thread = await _repository.createThread(
        ThreadInput(
          boardId: board.id,
          title: _titleController.text.trim(),
          body: _bodyController.text.trim().isEmpty
              ? null
              : _bodyController.text,
          contentFormat: ContentFormat.markdownV1,
          tags: _tags.toSet(),
          attachmentAssetIds: _attachmentAssetIds,
          poll: pollQuestion.isEmpty
              ? null
              : PollInput(question: pollQuestion, options: options.toSet()),
        ),
      );
      _isPublished = true;
      try {
        await _repository.deleteDraft(_draftKey);
      } on ApiFailure {
        if (mounted) {
          ScaffoldMessenger.of(
            context,
          ).showSnackBar(const SnackBar(content: Text('帖子已发布，但云端草稿清理失败')));
        }
      }
      if (mounted) {
        final GoRouter router = GoRouter.of(context);
        Navigator.of(context).pop();
        unawaited(router.push(AppRoutes.thread(thread.id)));
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _error = failure.message);
      }
    } finally {
      if (mounted) {
        setState(() => _isPublishing = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final Board? board = _selectedBoard;
    return Scaffold(
      appBar: AppBar(
        title: const Text('发布新帖'),
        actions: <Widget>[
          IconButton(
            tooltip: '保存云端草稿',
            onPressed: _isSavingDraft || !_hasContent ? null : _saveDraft,
            icon: const Icon(Icons.cloud_upload_outlined),
          ),
          TextButton(
            onPressed: _isPublishing ? null : _publish,
            child: Text(_isPublishing ? '发布中' : '发布'),
          ),
        ],
      ),
      body: Form(
        key: _formKey,
        child: ListView(
          padding: EdgeInsets.fromLTRB(
            20,
            12,
            20,
            24 + MediaQuery.viewInsetsOf(context).bottom,
          ),
          children: <Widget>[
            if (_isLoadingDraft) const LinearProgressIndicator(),
            if (_draftNotice case final String notice)
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(notice),
                      if (_remoteConflict != null) ...<Widget>[
                        const SizedBox(height: 8),
                        Wrap(
                          spacing: 8,
                          children: <Widget>[
                            OutlinedButton(
                              onPressed: _useRemote,
                              child: const Text('使用云端版本'),
                            ),
                            FilledButton.tonal(
                              onPressed: _keepLocal,
                              child: const Text('保留本地并覆盖'),
                            ),
                          ],
                        ),
                      ],
                    ],
                  ),
                ),
              ),
            DropdownButtonFormField<String>(
              initialValue: _boardId,
              decoration: const InputDecoration(labelText: '板块'),
              items: widget.boards
                  .map(
                    (Board item) => DropdownMenuItem<String>(
                      value: item.id,
                      enabled: item.canPost,
                      child: Text(
                        item.canPost
                            ? item.name
                            : '${item.name}（${_restriction(item)}）',
                      ),
                    ),
                  )
                  .toList(),
              onChanged: (String? value) {
                setState(() => _boardId = value);
                _scheduleDraftSave();
              },
              validator: (String? value) => value == null ? '请选择板块' : null,
            ),
            if (board != null && !board.canPost)
              Text(
                '你当前没有在此板块发帖的权限。',
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            const SizedBox(height: 14),
            TextFormField(
              controller: _titleController,
              maxLength: 120,
              decoration: const InputDecoration(labelText: '标题'),
              validator: (String? value) =>
                  value?.trim().isEmpty ?? true ? '请输入标题' : null,
            ),
            const SizedBox(height: 14),
            ForumMarkdownComposer(
              controller: _bodyController,
              label: '正文（Markdown）',
              minLines: 8,
              maxLines: 18,
              maxLength: 50000,
              helperText: '图片通过一次性 OSS 凭证直传；正文只保存 yourtj-asset 引用。',
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 10,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: <Widget>[
                if (_attachmentAssetIds.length < 8)
                  MediaUploadButton(
                    kind: MediaUploadKind.image,
                    usage: MediaUsage.forumThread,
                    onUploaded: _insertUploadedImage,
                  ),
                Text('${_attachmentAssetIds.length}/8 张图片'),
              ],
            ),
            const SizedBox(height: 14),
            TextFormField(
              controller: _tagsController,
              decoration: const InputDecoration(
                labelText: '标签',
                helperText: '最多 3 个，用空格分隔',
              ),
            ),
            const SizedBox(height: 14),
            TextFormField(
              controller: _pollQuestionController,
              maxLength: 200,
              decoration: const InputDecoration(labelText: '投票问题（可选）'),
            ),
            const SizedBox(height: 14),
            TextFormField(
              controller: _pollOptionsController,
              minLines: 3,
              maxLines: 8,
              decoration: const InputDecoration(
                labelText: '投票选项',
                helperText: '每行一个，2–20 个',
              ),
            ),
            if (_error case final String error) ...<Widget>[
              const SizedBox(height: 14),
              Semantics(
                liveRegion: true,
                child: Text(
                  error,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
            ],
            const SizedBox(height: 20),
            FilledButton.icon(
              onPressed: _isPublishing ? null : _publish,
              icon: _isPublishing
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.send_rounded),
              label: Text(_isPublishing ? '正在发布' : '发布'),
            ),
          ],
        ),
      ),
    );
  }

  String _restriction(Board board) {
    return switch (board.postingRestriction) {
      BoardPostingRestrictionEnum.trustLevel => '需信任等级 ${board.minTrustToPost}',
      BoardPostingRestrictionEnum.boardLocked => '板块已锁定',
      _ => '不可发帖',
    };
  }

  void _insertUploadedImage(CompletedMediaUpload upload) {
    final String separator =
        _bodyController.text.isEmpty || _bodyController.text.endsWith('\n')
        ? ''
        : '\n';
    _bodyController.text =
        '${_bodyController.text}$separator![图片](yourtj-asset:${upload.uploadId})\n';
    _bodyController.selection = TextSelection.collapsed(
      offset: _bodyController.text.length,
    );
    setState(() => _draftNotice = '图片已上传，发布前会保持为受控资源引用');
  }
}
