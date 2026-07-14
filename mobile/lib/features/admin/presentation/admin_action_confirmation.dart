import 'package:flutter/material.dart';

Future<String?> showAdminReasonConfirmation({
  required BuildContext context,
  required String actionLabel,
  required String impact,
  String? expectedVersion,
}) {
  return showDialog<String>(
    context: context,
    barrierDismissible: false,
    builder: (BuildContext context) => AdminReasonConfirmationDialog(
      actionLabel: actionLabel,
      impact: impact,
      expectedVersion: expectedVersion,
    ),
  );
}

class AdminReasonConfirmationDialog extends StatefulWidget {
  const AdminReasonConfirmationDialog({
    required this.actionLabel,
    required this.impact,
    this.expectedVersion,
    super.key,
  });

  final String actionLabel;
  final String impact;
  final String? expectedVersion;

  @override
  State<AdminReasonConfirmationDialog> createState() =>
      _AdminReasonConfirmationDialogState();
}

class _AdminReasonConfirmationDialogState
    extends State<AdminReasonConfirmationDialog> {
  final TextEditingController _reasonController = TextEditingController();
  bool _impactConfirmed = false;

  @override
  void dispose() {
    _reasonController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final String reason = _reasonController.text.trim();
    final bool canSubmit = reason.length >= 8 && _impactConfirmed;
    return AlertDialog(
      title: Text(widget.actionLabel),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Text(widget.impact),
              if (widget.expectedVersion != null) ...<Widget>[
                const SizedBox(height: 12),
                Text('已审阅版本：${widget.expectedVersion}'),
                const Text('若服务端返回冲突（HTTP 409），必须刷新证据后重新确认，不能覆盖并发修改。'),
              ],
              const SizedBox(height: 16),
              TextField(
                controller: _reasonController,
                minLines: 3,
                maxLines: 6,
                maxLength: 500,
                onChanged: (_) => setState(() {}),
                decoration: const InputDecoration(
                  labelText: '操作理由',
                  helperText: '至少 8 个字符；会写入不可变审计记录。',
                  alignLabelWithHint: true,
                ),
              ),
              CheckboxListTile(
                contentPadding: EdgeInsets.zero,
                value: _impactConfirmed,
                title: const Text('我已核对目标、证据与影响范围'),
                controlAffinity: ListTileControlAffinity.leading,
                onChanged: (bool? value) {
                  setState(() => _impactConfirmed = value ?? false);
                },
              ),
              const Text('需要近期认证的操作，应先通过服务器近期认证，再提交一次且不自动重试。'),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: canSubmit ? () => Navigator.of(context).pop(reason) : null,
          child: const Text('确认提交'),
        ),
      ],
    );
  }
}
