import 'package:flutter/material.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/widgets/app_state_views.dart';
import '../data/wallet_repository.dart';

enum WalletRecordSection { tasks, products, purchases, ledger }

class WalletRecordTabs extends StatefulWidget {
  const WalletRecordTabs({
    required this.snapshot,
    required this.accountId,
    required this.isMutating,
    required this.onAcceptTask,
    required this.onTaskAction,
    required this.onPurchase,
    required this.onPurchaseAction,
    super.key,
  });

  final WalletSnapshot snapshot;
  final String accountId;
  final bool isMutating;
  final ValueChanged<String> onAcceptTask;
  final void Function(Task task, TaskActionActionEnum action) onTaskAction;
  final ValueChanged<Product> onPurchase;
  final void Function(Purchase purchase, PurchaseActionActionEnum action)
  onPurchaseAction;

  @override
  State<WalletRecordTabs> createState() => _WalletRecordTabsState();
}

class _WalletRecordTabsState extends State<WalletRecordTabs> {
  WalletRecordSection _section = WalletRecordSection.tasks;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Semantics(
          container: true,
          label: '积分记录分类',
          child: Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              _sectionChip(WalletRecordSection.tasks, '悬赏任务'),
              _sectionChip(WalletRecordSection.products, '商品托管'),
              _sectionChip(WalletRecordSection.purchases, '我的订单'),
              _sectionChip(WalletRecordSection.ledger, '公开账本'),
            ],
          ),
        ),
        const SizedBox(height: 12),
        switch (_section) {
          WalletRecordSection.tasks => _TaskList(
            items: widget.snapshot.tasks.items,
            accountId: widget.accountId,
            isMutating: widget.isMutating,
            onAccept: widget.onAcceptTask,
            onAction: widget.onTaskAction,
          ),
          WalletRecordSection.products => _ProductList(
            items: widget.snapshot.products.items,
            accountId: widget.accountId,
            isMutating: widget.isMutating,
            onPurchase: widget.onPurchase,
          ),
          WalletRecordSection.purchases => _PurchaseList(
            items: widget.snapshot.purchases.items,
            accountId: widget.accountId,
            isMutating: widget.isMutating,
            onAction: widget.onPurchaseAction,
          ),
          WalletRecordSection.ledger => _LedgerList(
            items: widget.snapshot.ledger.items,
          ),
        },
      ],
    );
  }

  Widget _sectionChip(WalletRecordSection section, String label) {
    return ChoiceChip(
      selected: _section == section,
      label: Text(label),
      onSelected: (bool selected) {
        if (selected) {
          setState(() => _section = section);
        }
      },
    );
  }
}

class _TaskList extends StatelessWidget {
  const _TaskList({
    required this.items,
    required this.accountId,
    required this.isMutating,
    required this.onAccept,
    required this.onAction,
  });

  final List<Task> items;
  final String accountId;
  final bool isMutating;
  final ValueChanged<String> onAccept;
  final void Function(Task task, TaskActionActionEnum action) onAction;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const AppEmptyState(
        title: '暂无悬赏任务',
        description: '发布悬赏后，奖励积分会进入平台托管流程。',
      );
    }
    return Column(
      children: items
          .map(
            (Task task) => Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: _TaskCard(
                task: task,
                accountId: accountId,
                isMutating: isMutating,
                onAccept: onAccept,
                onAction: onAction,
              ),
            ),
          )
          .toList(growable: false),
    );
  }
}

class _TaskCard extends StatelessWidget {
  const _TaskCard({
    required this.task,
    required this.accountId,
    required this.isMutating,
    required this.onAccept,
    required this.onAction,
  });

  final Task task;
  final String accountId;
  final bool isMutating;
  final ValueChanged<String> onAccept;
  final void Function(Task task, TaskActionActionEnum action) onAction;

  @override
  Widget build(BuildContext context) {
    final String taskId = task.id;
    final TaskStatusEnum status = task.status;
    final bool isCreator = task.creatorId == accountId;
    final bool isAcceptor = task.acceptorId == accountId;
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        task.title,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 4),
                      Text(task.description ?? '无描述'),
                    ],
                  ),
                ),
                const SizedBox(width: 10),
                Chip(label: Text('${task.rewardAmount} 积分')),
              ],
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 6,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: <Widget>[
                Chip(label: Text(_taskStatus(status))),
                Text(_formatTime(task.createdAt)),
                if (task.contactInfo != null) Text('联系方式：${task.contactInfo}'),
              ],
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                if (status == TaskStatusEnum.open && !isCreator)
                  FilledButton.tonal(
                    onPressed: isMutating ? null : () => onAccept(taskId),
                    child: const Text('接单'),
                  ),
                if (status == TaskStatusEnum.inProgress && isAcceptor)
                  FilledButton.tonal(
                    onPressed: isMutating
                        ? null
                        : () => onAction(task, TaskActionActionEnum.submit),
                    child: const Text('提交完成'),
                  ),
                if (status == TaskStatusEnum.submitted && isCreator)
                  FilledButton(
                    onPressed: isMutating
                        ? null
                        : () => _confirmTaskAction(
                            context,
                            TaskActionActionEnum.confirm,
                            '确认完成并释放托管积分？',
                            '确认后奖励将进入接单者钱包，签名操作不会自动重试。',
                          ),
                    child: const Text('确认放款'),
                  ),
                if (isCreator &&
                    status != TaskStatusEnum.completed &&
                    status != TaskStatusEnum.cancelled)
                  OutlinedButton(
                    onPressed: isMutating
                        ? null
                        : () => _confirmTaskAction(
                            context,
                            TaskActionActionEnum.cancel,
                            '取消任务并退款？',
                            '服务端会按任务状态验证退款；该签名操作不会自动重试。',
                          ),
                    child: const Text('取消并退款'),
                  ),
                if (isAcceptor &&
                    (status == TaskStatusEnum.inProgress ||
                        status == TaskStatusEnum.submitted))
                  OutlinedButton(
                    onPressed: isMutating
                        ? null
                        : () => _confirmTaskAction(
                            context,
                            TaskActionActionEnum.reject,
                            '拒绝任务并退款？',
                            '拒绝后任务进入服务端定义的退款状态。',
                          ),
                    child: const Text('拒绝并退款'),
                  ),
                if (isCreator &&
                    (status == TaskStatusEnum.open ||
                        status == TaskStatusEnum.cancelled))
                  TextButton(
                    onPressed: isMutating
                        ? null
                        : () => onAction(task, TaskActionActionEnum.delete),
                    child: const Text('删除任务'),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _confirmTaskAction(
    BuildContext context,
    TaskActionActionEnum action,
    String title,
    String message,
  ) async {
    if (await _confirm(context, title, message) && context.mounted) {
      onAction(task, action);
    }
  }
}

class _ProductList extends StatelessWidget {
  const _ProductList({
    required this.items,
    required this.accountId,
    required this.isMutating,
    required this.onPurchase,
  });

  final List<Product> items;
  final String accountId;
  final bool isMutating;
  final ValueChanged<Product> onPurchase;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const AppEmptyState(title: '暂无在售商品');
    }
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final double itemWidth = constraints.maxWidth >= 760
            ? (constraints.maxWidth - 10) / 2
            : constraints.maxWidth;
        return Wrap(
          spacing: 10,
          runSpacing: 10,
          children: items
              .map(
                (Product product) => SizedBox(
                  width: itemWidth,
                  child: _ProductCard(
                    product: product,
                    accountId: accountId,
                    isMutating: isMutating,
                    onPurchase: onPurchase,
                  ),
                ),
              )
              .toList(growable: false),
        );
      },
    );
  }
}

class _ProductCard extends StatelessWidget {
  const _ProductCard({
    required this.product,
    required this.accountId,
    required this.isMutating,
    required this.onPurchase,
  });

  final Product product;
  final String accountId;
  final bool isMutating;
  final ValueChanged<Product> onPurchase;

  @override
  Widget build(BuildContext context) {
    final bool canBuy =
        product.id.isNotEmpty &&
        product.status == ProductStatusEnum.onSale &&
        product.sellerId.isNotEmpty &&
        product.sellerId != accountId &&
        product.price > 0 &&
        product.stock > 0;
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    product.title,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                Chip(label: Text('${product.price} 积分')),
              ],
            ),
            const SizedBox(height: 4),
            Text(product.description ?? '无描述'),
            const SizedBox(height: 10),
            Text(
              '${_productStatus(product.status)} · 库存 ${product.stock} · ${_formatTime(product.createdAt)}',
            ),
            const SizedBox(height: 10),
            FilledButton.tonal(
              onPressed: !canBuy || isMutating
                  ? null
                  : () async {
                      final bool confirmed = await _confirm(
                        context,
                        '购买并托管 ${product.price} 积分？',
                        '积分会先进入托管，完成交付后再释放给卖家。签名操作不会自动重试。',
                      );
                      if (confirmed && context.mounted) {
                        onPurchase(product);
                      }
                    },
              child: const Text('购买并托管'),
            ),
          ],
        ),
      ),
    );
  }
}

class _PurchaseList extends StatelessWidget {
  const _PurchaseList({
    required this.items,
    required this.accountId,
    required this.isMutating,
    required this.onAction,
  });

  final List<Purchase> items;
  final String accountId;
  final bool isMutating;
  final void Function(Purchase purchase, PurchaseActionActionEnum action)
  onAction;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const AppEmptyState(title: '暂无订单');
    }
    return Column(
      children: items
          .map(
            (Purchase purchase) => Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: _PurchaseCard(
                purchase: purchase,
                accountId: accountId,
                isMutating: isMutating,
                onAction: onAction,
              ),
            ),
          )
          .toList(growable: false),
    );
  }
}

class _PurchaseCard extends StatelessWidget {
  const _PurchaseCard({
    required this.purchase,
    required this.accountId,
    required this.isMutating,
    required this.onAction,
  });

  final Purchase purchase;
  final String accountId;
  final bool isMutating;
  final void Function(Purchase purchase, PurchaseActionActionEnum action)
  onAction;

  @override
  Widget build(BuildContext context) {
    final PurchaseStatusEnum status = purchase.status;
    final bool isBuyer = purchase.buyerId == accountId;
    final bool isSeller = purchase.sellerId == accountId;
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    '订单 ${purchase.id}',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                Chip(label: Text('${purchase.amount} 积分')),
              ],
            ),
            Text('商品 ${purchase.productId}'),
            if (purchase.deliveryInfo != null)
              Text('交付说明：${purchase.deliveryInfo}'),
            const SizedBox(height: 8),
            Text(
              '${_purchaseStatus(status)} · ${_formatTime(purchase.createdAt)}',
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                if (isSeller && status == PurchaseStatusEnum.pending)
                  FilledButton.tonal(
                    onPressed: isMutating
                        ? null
                        : () => onAction(
                            purchase,
                            PurchaseActionActionEnum.accept,
                          ),
                    child: const Text('卖家接单'),
                  ),
                if (isSeller && status == PurchaseStatusEnum.accepted)
                  FilledButton.tonal(
                    onPressed: isMutating
                        ? null
                        : () => onAction(
                            purchase,
                            PurchaseActionActionEnum.deliver,
                          ),
                    child: const Text('标记交付'),
                  ),
                if (isBuyer && status == PurchaseStatusEnum.delivered)
                  FilledButton(
                    onPressed: isMutating
                        ? null
                        : () => _confirmedAction(
                            context,
                            PurchaseActionActionEnum.confirm,
                            '确认收货并释放积分？',
                            '确认后托管积分将进入卖家钱包，此签名操作不会自动重试。',
                          ),
                    child: const Text('确认完成'),
                  ),
                if (isBuyer &&
                    (status == PurchaseStatusEnum.pending ||
                        status == PurchaseStatusEnum.accepted))
                  OutlinedButton(
                    onPressed: isMutating
                        ? null
                        : () => _confirmedAction(
                            context,
                            PurchaseActionActionEnum.cancel,
                            '取消订单并退款？',
                            '服务端会验证订单状态并退回托管积分，此签名操作不会自动重试。',
                          ),
                    child: const Text('取消并退款'),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _confirmedAction(
    BuildContext context,
    PurchaseActionActionEnum action,
    String title,
    String message,
  ) async {
    if (await _confirm(context, title, message) && context.mounted) {
      onAction(purchase, action);
    }
  }
}

class _LedgerList extends StatelessWidget {
  const _LedgerList({required this.items});

  final List<LedgerEntry> items;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      return const AppEmptyState(title: '暂无账本记录');
    }
    return Column(
      children: items
          .map(
            (LedgerEntry entry) => Card(
              margin: const EdgeInsets.only(bottom: 8),
              child: ListTile(
                leading: const Icon(Icons.receipt_long_outlined),
                title: Text(
                  '${_ledgerType(entry.type)} · ${entry.amount ?? 0} 积分',
                ),
                subtitle: Text(
                  '#${entry.seq ?? 0} · ${_formatTime(entry.createdAt)} · ${_shortHash(entry.hash)}\n'
                  '${entry.fromAccount ?? 'system'} → ${entry.toAccount ?? 'escrow'}',
                ),
                isThreeLine: true,
              ),
            ),
          )
          .toList(growable: false),
    );
  }
}

Future<bool> _confirm(
  BuildContext context,
  String title,
  String message,
) async {
  return await showDialog<bool>(
        context: context,
        builder: (BuildContext context) => AlertDialog(
          title: Text(title),
          content: Text(message),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('返回'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('确认'),
            ),
          ],
        ),
      ) ??
      false;
}

String _formatTime(int? seconds) {
  if (seconds == null) {
    return '时间未知';
  }
  final DateTime time = DateTime.fromMillisecondsSinceEpoch(
    seconds * 1000,
    isUtc: true,
  ).toLocal();
  String twoDigits(int value) => value.toString().padLeft(2, '0');
  return '${time.year}-${twoDigits(time.month)}-${twoDigits(time.day)} '
      '${twoDigits(time.hour)}:${twoDigits(time.minute)}';
}

String _shortHash(String? value) {
  if (value == null || value.isEmpty) {
    return '—';
  }
  return value.length <= 12
      ? value
      : '${value.substring(0, 6)}…${value.substring(value.length - 6)}';
}

String _taskStatus(TaskStatusEnum? status) => switch (status) {
  TaskStatusEnum.open => '待接单',
  TaskStatusEnum.inProgress => '进行中',
  TaskStatusEnum.submitted => '待确认',
  TaskStatusEnum.completed => '已完成',
  TaskStatusEnum.cancelled => '已取消',
  _ => '状态未知',
};

String _productStatus(ProductStatusEnum? status) => switch (status) {
  ProductStatusEnum.onSale => '在售',
  ProductStatusEnum.offSale => '已下架',
  ProductStatusEnum.soldOut => '售罄',
  _ => '状态未知',
};

String _purchaseStatus(PurchaseStatusEnum? status) => switch (status) {
  PurchaseStatusEnum.pending => '待接单',
  PurchaseStatusEnum.accepted => '已接单',
  PurchaseStatusEnum.delivered => '已交付',
  PurchaseStatusEnum.completed => '已完成',
  PurchaseStatusEnum.cancelled => '已取消',
  _ => '状态未知',
};

String _ledgerType(LedgerEntryTypeEnum? type) => switch (type) {
  LedgerEntryTypeEnum.mint => '贡献铸造',
  LedgerEntryTypeEnum.tip => '内容打赏',
  LedgerEntryTypeEnum.escrowHold => '托管锁定',
  LedgerEntryTypeEnum.escrowRelease => '托管释放',
  _ => '未知记录',
};
