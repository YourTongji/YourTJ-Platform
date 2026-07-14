import 'package:flutter/material.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/validation/identity_inputs.dart';

class NewConversationDraft {
  const NewConversationDraft({
    required this.handle,
    required this.requestMessage,
  });

  final String handle;
  final String requestMessage;
}

Future<NewConversationDraft?> showNewConversationDialog(BuildContext context) {
  return showDialog<NewConversationDraft>(
    context: context,
    builder: (BuildContext context) => const _NewConversationDialog(),
  );
}

class _NewConversationDialog extends StatefulWidget {
  const _NewConversationDialog();

  @override
  State<_NewConversationDialog> createState() => _NewConversationDialogState();
}

class _NewConversationDialogState extends State<_NewConversationDialog> {
  final TextEditingController _handleController = TextEditingController();
  final TextEditingController _requestController = TextEditingController();
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();

  @override
  void dispose() {
    _handleController.dispose();
    _requestController.dispose();
    super.dispose();
  }

  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    Navigator.of(context).pop(
      NewConversationDraft(
        handle: _handleController.text.trim(),
        requestMessage: _requestController.text.trim(),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('发起私信'),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: SingleChildScrollView(
          child: Form(
            key: _formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                TextFormField(
                  controller: _handleController,
                  textInputAction: TextInputAction.next,
                  maxLength: 30,
                  decoration: const InputDecoration(
                    labelText: '对方公开用户名',
                    prefixText: '@',
                  ),
                  validator: (String? value) {
                    if (!IdentityInputs.isValidPublicHandle(
                      value?.trim() ?? '',
                    )) {
                      return '请输入 3–30 位小写字母、数字、点、下划线或短横线';
                    }
                    return null;
                  },
                ),
                const SizedBox(height: 12),
                TextFormField(
                  controller: _requestController,
                  minLines: 3,
                  maxLines: 6,
                  maxLength: 1000,
                  decoration: const InputDecoration(
                    labelText: '陌生联系附言',
                    helperText: '若对方不接受直接私信，这会成为请求阶段唯一的一条消息；不要填写校园身份等不必要信息。',
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(onPressed: _submit, child: const Text('继续')),
      ],
    );
  }
}

class DmReportDraft {
  const DmReportDraft({required this.reason, required this.note});

  final DmReportInputReasonEnum reason;
  final String? note;
}

Future<DmReportDraft?> showDmReportDialog(
  BuildContext context, {
  required bool isRequest,
}) {
  return showDialog<DmReportDraft>(
    context: context,
    builder: (BuildContext context) => _DmReportDialog(isRequest: isRequest),
  );
}

class _DmReportDialog extends StatefulWidget {
  const _DmReportDialog({required this.isRequest});

  final bool isRequest;

  @override
  State<_DmReportDialog> createState() => _DmReportDialogState();
}

class _DmReportDialogState extends State<_DmReportDialog> {
  final TextEditingController _noteController = TextEditingController();
  DmReportInputReasonEnum _reason = DmReportInputReasonEnum.spam;

  @override
  void dispose() {
    _noteController.dispose();
    super.dispose();
  }

  void _submit() {
    final String note = _noteController.text.trim();
    Navigator.of(
      context,
    ).pop(DmReportDraft(reason: _reason, note: note.isEmpty ? null : note));
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.isRequest ? '举报消息请求' : '举报这条私信'),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              const Text('审核人员只会看到被举报的具体消息、有限上下文和你的说明，不会获得通用私信浏览权限。'),
              const SizedBox(height: 16),
              DropdownButtonFormField<DmReportInputReasonEnum>(
                initialValue: _reason,
                decoration: const InputDecoration(labelText: '举报类型'),
                items: DmReportInputReasonEnum.values
                    .where(
                      (DmReportInputReasonEnum value) =>
                          value !=
                          DmReportInputReasonEnum.unknownDefaultOpenApi,
                    )
                    .map(
                      (DmReportInputReasonEnum value) =>
                          DropdownMenuItem<DmReportInputReasonEnum>(
                            value: value,
                            child: Text(_reportReasonLabel(value)),
                          ),
                    )
                    .toList(growable: false),
                onChanged: (DmReportInputReasonEnum? value) {
                  if (value != null) {
                    setState(() => _reason = value);
                  }
                },
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _noteController,
                minLines: 2,
                maxLines: 5,
                maxLength: 1000,
                decoration: const InputDecoration(labelText: '补充说明（可选）'),
              ),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(onPressed: _submit, child: const Text('提交举报')),
      ],
    );
  }
}

String _reportReasonLabel(DmReportInputReasonEnum reason) => switch (reason) {
  DmReportInputReasonEnum.spam => '垃圾信息',
  DmReportInputReasonEnum.abuse => '辱骂攻击',
  DmReportInputReasonEnum.harassment => '骚扰',
  DmReportInputReasonEnum.fraud => '欺诈',
  DmReportInputReasonEnum.illegal => '违法内容',
  DmReportInputReasonEnum.other => '其他',
  DmReportInputReasonEnum.unknownDefaultOpenApi => '未知类型',
};
