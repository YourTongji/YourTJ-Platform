import 'package:flutter/material.dart';

import '../domain/admin_mutations.dart';

Future<AdminMutationSubmission?> showAdminMutationDialog({
  required BuildContext context,
  required AdminMutationAction action,
}) {
  return showDialog<AdminMutationSubmission>(
    context: context,
    barrierDismissible: false,
    builder: (BuildContext context) => AdminMutationDialog(action: action),
  );
}

class AdminMutationDialog extends StatefulWidget {
  const AdminMutationDialog({required this.action, super.key});

  final AdminMutationAction action;

  @override
  State<AdminMutationDialog> createState() => _AdminMutationDialogState();
}

class _AdminMutationDialogState extends State<AdminMutationDialog> {
  final TextEditingController _reasonController = TextEditingController();
  final Map<String, TextEditingController> _controllers =
      <String, TextEditingController>{};
  final Map<String, bool> _booleans = <String, bool>{};
  final Map<String, String> _choices = <String, String>{};
  bool _confirmed = false;

  @override
  void initState() {
    super.initState();
    for (final AdminMutationField field in widget.action.fields) {
      switch (field.kind) {
        case AdminMutationFieldKind.boolean:
          _booleans[field.key] = field.initialValue == 'true';
        case AdminMutationFieldKind.choice:
          _choices[field.key] = field.initialValue.isNotEmpty
              ? field.initialValue
              : field.options.firstOrNull?.value ?? '';
        case AdminMutationFieldKind.text ||
            AdminMutationFieldKind.multiline ||
            AdminMutationFieldKind.integer ||
            AdminMutationFieldKind.decimal:
          _controllers[field.key] = TextEditingController(
            text: field.initialValue,
          );
      }
    }
  }

  @override
  void dispose() {
    _reasonController.dispose();
    for (final TextEditingController controller in _controllers.values) {
      controller.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final bool isValid = _isValid();
    return AlertDialog(
      title: Text(widget.action.label),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Text(widget.action.impact),
              if (widget.action.expectedVersion != null) ...<Widget>[
                const SizedBox(height: 8),
                Text('已审阅版本：${widget.action.expectedVersion}'),
                const Text('HTTP 409 时不会覆盖并发修改；请刷新证据后重新填写并确认。'),
              ],
              const SizedBox(height: 16),
              for (final AdminMutationField field
                  in widget.action.fields) ...<Widget>[
                _field(field),
                const SizedBox(height: 12),
              ],
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
                value: _confirmed,
                title: const Text('我已核对目标、证据、权限边界与影响范围'),
                subtitle: const Text('提交前会检查服务器近期认证；请求不会自动重试。'),
                controlAffinity: ListTileControlAffinity.leading,
                onChanged: (bool? value) {
                  setState(() => _confirmed = value ?? false);
                },
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
        FilledButton(
          style: widget.action.isDestructive
              ? FilledButton.styleFrom(
                  backgroundColor: Theme.of(context).colorScheme.error,
                  foregroundColor: Theme.of(context).colorScheme.onError,
                )
              : null,
          onPressed: isValid ? _submit : null,
          child: const Text('确认提交'),
        ),
      ],
    );
  }

  Widget _field(AdminMutationField field) {
    switch (field.kind) {
      case AdminMutationFieldKind.boolean:
        return SwitchListTile(
          contentPadding: EdgeInsets.zero,
          value: _booleans[field.key] ?? false,
          title: Text(field.label),
          subtitle: field.helperText == null ? null : Text(field.helperText!),
          onChanged: (bool value) {
            setState(() => _booleans[field.key] = value);
          },
        );
      case AdminMutationFieldKind.choice:
        return DropdownButtonFormField<String>(
          initialValue: _choices[field.key],
          decoration: InputDecoration(
            labelText: field.label,
            helperText: field.helperText,
          ),
          items: field.options
              .map(
                (AdminMutationOption option) => DropdownMenuItem<String>(
                  value: option.value,
                  child: Text(option.label),
                ),
              )
              .toList(growable: false),
          onChanged: (String? value) {
            if (value != null) {
              setState(() => _choices[field.key] = value);
            }
          },
        );
      case AdminMutationFieldKind.text ||
          AdminMutationFieldKind.multiline ||
          AdminMutationFieldKind.integer ||
          AdminMutationFieldKind.decimal:
        final bool isNumber =
            field.kind == AdminMutationFieldKind.integer ||
            field.kind == AdminMutationFieldKind.decimal;
        return TextField(
          controller: _controllers[field.key],
          minLines: field.kind == AdminMutationFieldKind.multiline ? 3 : 1,
          maxLines: field.kind == AdminMutationFieldKind.multiline ? 6 : 1,
          keyboardType: isNumber
              ? TextInputType.numberWithOptions(
                  decimal: field.kind == AdminMutationFieldKind.decimal,
                  signed: false,
                )
              : TextInputType.text,
          onChanged: (_) => setState(() {}),
          decoration: InputDecoration(
            labelText: field.label,
            helperText: field.helperText,
          ),
        );
    }
  }

  bool _isValid() {
    if (!_confirmed || _reasonController.text.trim().length < 8) {
      return false;
    }
    for (final AdminMutationField field in widget.action.fields) {
      if (field.mustBeTrue && !(_booleans[field.key] ?? false)) {
        return false;
      }
      if (!field.isRequired) {
        continue;
      }
      final String value = switch (field.kind) {
        AdminMutationFieldKind.boolean =>
          (_booleans[field.key] ?? false).toString(),
        AdminMutationFieldKind.choice => _choices[field.key] ?? '',
        _ => _controllers[field.key]?.text.trim() ?? '',
      };
      if (value.isEmpty) {
        return false;
      }
    }
    return true;
  }

  void _submit() {
    final Map<String, String> values = <String, String>{};
    for (final AdminMutationField field in widget.action.fields) {
      values[field.key] = switch (field.kind) {
        AdminMutationFieldKind.boolean =>
          (_booleans[field.key] ?? false).toString(),
        AdminMutationFieldKind.choice => _choices[field.key] ?? '',
        _ => _controllers[field.key]?.text.trim() ?? '',
      };
    }
    Navigator.of(context).pop(
      AdminMutationSubmission(
        reason: _reasonController.text.trim(),
        values: values,
      ),
    );
  }
}
