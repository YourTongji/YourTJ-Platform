import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../data/wallet_repository.dart';

Future<TaskInput?> showCreateTaskDialog(BuildContext context) {
  return showDialog<TaskInput>(
    context: context,
    builder: (BuildContext context) => const _CreateTaskDialog(),
  );
}

Future<ProductInput?> showCreateProductDialog(BuildContext context) {
  return showDialog<ProductInput>(
    context: context,
    builder: (BuildContext context) => const _CreateProductDialog(),
  );
}

Future<bool> showLegacyClaimDialog(
  BuildContext context,
  WalletRepository repository,
) async {
  return await showDialog<bool>(
        context: context,
        builder: (BuildContext context) =>
            _LegacyClaimDialog(repository: repository),
      ) ??
      false;
}

class WalletTipComposer extends StatefulWidget {
  const WalletTipComposer({
    required this.isSubmitting,
    required this.onSubmit,
    super.key,
  });

  final bool isSubmitting;
  final ValueChanged<TipInput> onSubmit;

  @override
  State<WalletTipComposer> createState() => _WalletTipComposerState();
}

class _WalletTipComposerState extends State<WalletTipComposer> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _account = TextEditingController();
  final TextEditingController _amount = TextEditingController(text: '1');
  final TextEditingController _target = TextEditingController();
  TipInputTargetTypeEnum _targetType = TipInputTargetTypeEnum.thread;

  @override
  void dispose() {
    _account.dispose();
    _amount.dispose();
    _target.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Form(
          key: _formKey,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text('内容打赏', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 6),
              const Text('打赏必须绑定到 review、thread 或 comment；这里不提供自由转账。'),
              const SizedBox(height: 14),
              Wrap(
                spacing: 12,
                runSpacing: 12,
                crossAxisAlignment: WrapCrossAlignment.end,
                children: <Widget>[
                  SizedBox(
                    width: 230,
                    child: TextFormField(
                      controller: _account,
                      decoration: const InputDecoration(labelText: '收款账号 ID'),
                      validator: _required,
                    ),
                  ),
                  SizedBox(
                    width: 120,
                    child: TextFormField(
                      controller: _amount,
                      keyboardType: TextInputType.number,
                      decoration: const InputDecoration(labelText: '积分金额'),
                      validator: _positiveInteger,
                    ),
                  ),
                  SizedBox(
                    width: 160,
                    child: DropdownButtonFormField<TipInputTargetTypeEnum>(
                      initialValue: _targetType,
                      decoration: const InputDecoration(labelText: '内容类型'),
                      items: const <DropdownMenuItem<TipInputTargetTypeEnum>>[
                        DropdownMenuItem(
                          value: TipInputTargetTypeEnum.thread,
                          child: Text('主题'),
                        ),
                        DropdownMenuItem(
                          value: TipInputTargetTypeEnum.comment,
                          child: Text('评论'),
                        ),
                        DropdownMenuItem(
                          value: TipInputTargetTypeEnum.review,
                          child: Text('评课'),
                        ),
                      ],
                      onChanged: (TipInputTargetTypeEnum? value) {
                        if (value != null) {
                          setState(() => _targetType = value);
                        }
                      },
                    ),
                  ),
                  SizedBox(
                    width: 230,
                    child: TextFormField(
                      controller: _target,
                      decoration: const InputDecoration(labelText: '内容 ID'),
                      validator: _required,
                    ),
                  ),
                  FilledButton.icon(
                    onPressed: widget.isSubmitting ? null : _submit,
                    icon: const Icon(Icons.volunteer_activism_outlined),
                    label: const Text('确认打赏'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    widget.onSubmit(
      TipInput(
        toAccountId: _account.text.trim(),
        amount: int.parse(_amount.text),
        targetType: _targetType,
        targetId: _target.text.trim(),
      ),
    );
  }
}

class _CreateTaskDialog extends StatefulWidget {
  const _CreateTaskDialog();

  @override
  State<_CreateTaskDialog> createState() => _CreateTaskDialogState();
}

class _CreateTaskDialogState extends State<_CreateTaskDialog> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _title = TextEditingController();
  final TextEditingController _reward = TextEditingController(text: '10');
  final TextEditingController _description = TextEditingController();
  final TextEditingController _contact = TextEditingController();

  @override
  void dispose() {
    _title.dispose();
    _reward.dispose();
    _description.dispose();
    _contact.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('发布悬赏任务'),
      content: SizedBox(
        width: 520,
        child: Form(
          key: _formKey,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                TextFormField(
                  controller: _title,
                  autofocus: true,
                  maxLength: 120,
                  decoration: const InputDecoration(labelText: '标题'),
                  validator: _required,
                ),
                const SizedBox(height: 10),
                TextFormField(
                  controller: _reward,
                  keyboardType: TextInputType.number,
                  decoration: const InputDecoration(labelText: '奖励积分'),
                  validator: _positiveInteger,
                ),
                const SizedBox(height: 10),
                TextFormField(
                  controller: _description,
                  maxLength: 2000,
                  maxLines: 4,
                  decoration: const InputDecoration(labelText: '描述（可选）'),
                ),
                const SizedBox(height: 10),
                TextFormField(
                  controller: _contact,
                  maxLength: 200,
                  decoration: const InputDecoration(labelText: '接单后可见联系方式（可选）'),
                ),
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(onPressed: _submit, child: const Text('签名并发布')),
      ],
    );
  }

  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    Navigator.pop(
      context,
      TaskInput(
        title: _title.text.trim(),
        rewardAmount: int.parse(_reward.text),
        description: _optional(_description.text),
        contactInfo: _optional(_contact.text),
      ),
    );
  }
}

class _CreateProductDialog extends StatefulWidget {
  const _CreateProductDialog();

  @override
  State<_CreateProductDialog> createState() => _CreateProductDialogState();
}

class _CreateProductDialogState extends State<_CreateProductDialog> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _title = TextEditingController();
  final TextEditingController _price = TextEditingController(text: '10');
  final TextEditingController _stock = TextEditingController(text: '1');
  final TextEditingController _description = TextEditingController();
  final TextEditingController _delivery = TextEditingController();

  @override
  void dispose() {
    _title.dispose();
    _price.dispose();
    _stock.dispose();
    _description.dispose();
    _delivery.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('上架商品'),
      content: SizedBox(
        width: 520,
        child: Form(
          key: _formKey,
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                TextFormField(
                  controller: _title,
                  autofocus: true,
                  maxLength: 120,
                  decoration: const InputDecoration(labelText: '标题'),
                  validator: _required,
                ),
                const SizedBox(height: 10),
                Row(
                  children: <Widget>[
                    Expanded(
                      child: TextFormField(
                        controller: _price,
                        keyboardType: TextInputType.number,
                        decoration: const InputDecoration(labelText: '价格'),
                        validator: _positiveInteger,
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: TextFormField(
                        controller: _stock,
                        keyboardType: TextInputType.number,
                        decoration: const InputDecoration(labelText: '库存'),
                        validator: _nonNegativeInteger,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                TextFormField(
                  controller: _description,
                  maxLength: 2000,
                  maxLines: 4,
                  decoration: const InputDecoration(labelText: '描述（可选）'),
                ),
                const SizedBox(height: 10),
                TextFormField(
                  controller: _delivery,
                  maxLength: 500,
                  decoration: const InputDecoration(labelText: '交付说明（可选）'),
                ),
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(onPressed: _submit, child: const Text('上架')),
      ],
    );
  }

  void _submit() {
    if (!(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    Navigator.pop(
      context,
      ProductInput(
        title: _title.text.trim(),
        price: int.parse(_price.text),
        stock: int.parse(_stock.text),
        description: _optional(_description.text),
        deliveryInfo: _optional(_delivery.text),
      ),
    );
  }
}

class _LegacyClaimDialog extends StatefulWidget {
  const _LegacyClaimDialog({required this.repository});

  final WalletRepository repository;

  @override
  State<_LegacyClaimDialog> createState() => _LegacyClaimDialogState();
}

class _LegacyClaimDialogState extends State<_LegacyClaimDialog> {
  final TextEditingController _legacyHash = TextEditingController();
  final TextEditingController _signature = TextEditingController();
  WalletClaimChallenge? _challenge;
  String? _error;
  bool _isSubmitting = false;

  @override
  void dispose() {
    _legacyHash.dispose();
    _signature.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('认领旧钱包'),
      content: SizedBox(
        width: 560,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              const Text('使用旧钱包对服务端挑战签名后合并余额；平台不会接触旧私钥或 PIN。'),
              const SizedBox(height: 14),
              FilledButton.tonal(
                onPressed: _isSubmitting ? null : _requestChallenge,
                child: Text(_challenge == null ? '获取 10 分钟挑战' : '重新获取挑战'),
              ),
              if (_challenge != null) ...<Widget>[
                const SizedBox(height: 12),
                SelectableText('challengeId: ${_challenge!.challengeId}'),
                SelectableText('nonce: ${_challenge!.nonce}'),
                const SizedBox(height: 12),
                TextField(
                  controller: _legacyHash,
                  maxLength: 64,
                  keyboardType: TextInputType.visiblePassword,
                  inputFormatters: <TextInputFormatter>[
                    FilteringTextInputFormatter.allow(RegExp('[0-9a-f]')),
                    LengthLimitingTextInputFormatter(64),
                  ],
                  decoration: const InputDecoration(
                    labelText: 'legacyUserHash',
                  ),
                ),
                const SizedBox(height: 10),
                TextField(
                  controller: _signature,
                  minLines: 2,
                  maxLines: 4,
                  maxLength: 88,
                  keyboardType: TextInputType.visiblePassword,
                  inputFormatters: <TextInputFormatter>[
                    FilteringTextInputFormatter.allow(
                      RegExp(r'[A-Za-z0-9+/=]'),
                    ),
                    LengthLimitingTextInputFormatter(88),
                  ],
                  decoration: const InputDecoration(labelText: '旧钱包签名'),
                ),
              ],
              if (_error != null) ...<Widget>[
                const SizedBox(height: 12),
                Text(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _isSubmitting ? null : () => Navigator.pop(context, false),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _challenge == null || _isSubmitting ? null : _submitClaim,
          child: const Text('验证并合并'),
        ),
      ],
    );
  }

  Future<void> _requestChallenge() async {
    setState(() {
      _isSubmitting = true;
      _error = null;
    });
    try {
      final WalletClaimChallenge challenge = await widget.repository
          .createClaimChallenge();
      if (mounted) {
        setState(() {
          _challenge = challenge;
          _signature.clear();
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _error = failure.message);
      }
    } finally {
      if (mounted) {
        setState(() => _isSubmitting = false);
      }
    }
  }

  Future<void> _submitClaim() async {
    final String legacyHash = _legacyHash.text.trim();
    final String signature = _signature.text.trim();
    if (!RegExp(r'^[0-9a-f]{64}$').hasMatch(legacyHash) ||
        !RegExp(r'^[A-Za-z0-9+/]{86}==$').hasMatch(signature)) {
      setState(() => _error = '旧钱包标识或签名格式无效');
      return;
    }
    setState(() {
      _isSubmitting = true;
      _error = null;
    });
    try {
      await widget.repository.claimLegacyWallet(
        legacyUserHash: legacyHash,
        challengeId: _challenge!.challengeId,
        signature: signature,
      );
      if (mounted) {
        Navigator.pop(context, true);
      }
    } on ApiFailure {
      if (mounted) {
        setState(() {
          _challenge = null;
          _signature.clear();
          _error = '认领未完成；本次挑战已失效，请重新获取并签名';
        });
      }
    } finally {
      if (mounted) {
        setState(() => _isSubmitting = false);
      }
    }
  }
}

String? _required(String? value) {
  if (value == null || value.trim().isEmpty) {
    return '请填写此项';
  }
  return null;
}

String? _positiveInteger(String? value) {
  final int? parsed = int.tryParse(value ?? '');
  if (parsed == null || parsed <= 0) {
    return '请输入大于 0 的整数';
  }
  return null;
}

String? _nonNegativeInteger(String? value) {
  final int? parsed = int.tryParse(value ?? '');
  if (parsed == null || parsed < 0) {
    return '请输入不小于 0 的整数';
  }
  return null;
}

String? _optional(String value) {
  final String trimmed = value.trim();
  return trimmed.isEmpty ? null : trimmed;
}
