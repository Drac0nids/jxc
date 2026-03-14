import 'package:flutter/material.dart';

enum NoticeTone { info, success, warning, error }

class BrandHeroBanner extends StatelessWidget {
  const BrandHeroBanner({
    super.key,
    required this.title,
    required this.subtitle,
    required this.icon,
    this.trailing,
    this.gradientSeedColor,
  });

  final String title;
  final String subtitle;
  final IconData icon;
  final Widget? trailing;
  final Color? gradientSeedColor;

  @override
  Widget build(BuildContext context) {
    final ColorScheme scheme = Theme.of(context).colorScheme;
    final Color mixColor = gradientSeedColor ?? scheme.primary;

    return Container(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(20),
        gradient: LinearGradient(
          colors: <Color>[
            mixColor,
            mixColor.withValues(alpha: 0.8),
          ],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
      ),
      padding: const EdgeInsets.all(16),
      child: Row(
        children: <Widget>[
          Container(
            width: 44,
            height: 44,
            decoration: BoxDecoration(
              color: Colors.white.withValues(alpha: 0.2),
              borderRadius: BorderRadius.circular(14),
            ),
            child: Icon(icon, color: Colors.white),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  title,
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 17,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  subtitle,
                  style: TextStyle(
                    color: Colors.white.withValues(alpha: 0.92),
                    fontSize: 12,
                  ),
                ),
              ],
            ),
          ),
          if (trailing != null) ...<Widget>[
            const SizedBox(width: 8),
            trailing!,
          ],
        ],
      ),
    );
  }
}

class SectionCard extends StatelessWidget {
  const SectionCard({
    super.key,
    required this.title,
    this.subtitle,
    this.action,
    this.footer,
    required this.child,
  });

  final String title;
  final String? subtitle;
  final Widget? action;
  final Widget? footer;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final TextTheme textTheme = Theme.of(context).textTheme;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        title,
                        style: textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      if (subtitle != null) ...<Widget>[
                        const SizedBox(height: 4),
                        Text(
                          subtitle!,
                          style: textTheme.bodySmall,
                        ),
                      ],
                    ],
                  ),
                ),
                if (action != null) ...<Widget>[
                  const SizedBox(width: 8),
                  action!,
                ],
              ],
            ),
            child,
            if (footer != null) ...<Widget>[
              const SizedBox(height: 10),
              footer!,
            ],
          ],
        ),
      ),
    );
  }
}

class StatusNotice extends StatelessWidget {
  const StatusNotice({
    super.key,
    required this.message,
    required this.tone,
  });

  final String message;
  final NoticeTone tone;

  @override
  Widget build(BuildContext context) {
    final ColorScheme scheme = Theme.of(context).colorScheme;
    final (Color bg, Color fg, IconData icon) = switch (tone) {
      NoticeTone.info => (
          scheme.primaryContainer,
          scheme.onPrimaryContainer,
          Icons.info_outline,
        ),
      NoticeTone.success => (
          const Color(0xFFD1FAE5),
          const Color(0xFF065F46),
          Icons.check_circle_outline,
        ),
      NoticeTone.warning => (
          const Color(0xFFFEF3C7),
          const Color(0xFF92400E),
          Icons.warning_amber_outlined,
        ),
      NoticeTone.error => (
          scheme.errorContainer,
          scheme.onErrorContainer,
          Icons.error_outline,
        ),
    };

    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(14),
      ),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      child: Row(
        children: <Widget>[
          Icon(icon, color: fg, size: 18),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              message,
              style: TextStyle(color: fg, fontWeight: FontWeight.w600),
            ),
          ),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Shared Filter Widgets — 可被所有带日期/分页筛选的页面复用
// ════════════════════════════════════════════════════════════════════════════

/// 快捷日期选择 Chip（今日/本周/本月/近30天）
class QuickDateChip extends StatelessWidget {
  const QuickDateChip({super.key, required this.label, required this.onTap});
  final String label;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(right: 6),
      child: ActionChip(
        label: Text(label, style: const TextStyle(fontSize: 12)),
        onPressed: onTap,
        visualDensity: VisualDensity.compact,
        padding: const EdgeInsets.symmetric(horizontal: 4),
      ),
    );
  }
}

/// 快捷日期 Chip 行：今日 / 本周 / 本月 / 近30天
/// [onApply] 回调传入 (startDate, endDate)
class QuickDateChipsRow extends StatelessWidget {
  const QuickDateChipsRow({
    super.key,
    required this.onApply,
    this.disabled = false,
  });
  final void Function(DateTime start, DateTime end) onApply;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    final now = DateUtils.dateOnly(DateTime.now());
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: Row(
        children: <Widget>[
          QuickDateChip(
            label: '今日',
            onTap: disabled ? null : () => onApply(now, now),
          ),
          QuickDateChip(
            label: '本周',
            onTap: disabled
                ? null
                : () {
                    final start =
                        now.subtract(Duration(days: now.weekday - 1));
                    onApply(start, now);
                  },
          ),
          QuickDateChip(
            label: '本月',
            onTap: disabled
                ? null
                : () {
                    final start = DateTime(now.year, now.month, 1);
                    onApply(start, now);
                  },
          ),
          QuickDateChip(
            label: '近30天',
            onTap: disabled
                ? null
                : () =>
                    onApply(now.subtract(const Duration(days: 29)), now),
          ),
        ],
      ),
    );
  }
}

/// 日期选择按钮，点击打开 DatePicker
class DatePickerField extends StatelessWidget {
  const DatePickerField({
    super.key,
    required this.label,
    required this.value,
    required this.onTap,
  });
  final String label;
  final String value;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: InputDecorator(
        decoration: InputDecoration(
          labelText: label,
          border: const OutlineInputBorder(),
          isDense: true,
          suffixIcon: const Icon(Icons.calendar_month_outlined, size: 18),
        ),
        child: Text(value, style: const TextStyle(fontSize: 14)),
      ),
    );
  }
}

/// SegmentedButton 选择每页条数
/// [options]   可选条数列表，如 [10, 20, 50]
/// [selected]  当前选中的条数值
/// [onChanged] 回调传入选中值
class PageSizeSegmented extends StatelessWidget {
  const PageSizeSegmented({
    super.key,
    required this.options,
    required this.selected,
    required this.onChanged,
    this.disabled = false,
  });
  final List<int> options;
  final int selected;
  final void Function(int) onChanged;
  final bool disabled;

  @override
  Widget build(BuildContext context) {
    return SegmentedButton<int>(
      segments: options
          .map((n) => ButtonSegment<int>(value: n, label: Text('$n')))
          .toList(),
      selected: <int>{selected},
      onSelectionChanged:
          disabled ? null : (s) => onChanged(s.first),
      style: const ButtonStyle(
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
        visualDensity: VisualDensity.compact,
      ),
    );
  }
}

/// 分页导航栏：上一页  第 X/Y 页  下一页
class PaginationBar extends StatelessWidget {
  const PaginationBar({
    super.key,
    required this.page,
    required this.totalPages,
    required this.loading,
    required this.onPrev,
    required this.onNext,
  });
  final int page;
  final int totalPages;
  final bool loading;
  final VoidCallback? onPrev;
  final VoidCallback? onNext;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: <Widget>[
        OutlinedButton.icon(
          onPressed: loading ? null : onPrev,
          icon: const Icon(Icons.chevron_left, size: 18),
          label: const Text('上一页'),
        ),
        const Spacer(),
        Container(
          padding:
              const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
          decoration: BoxDecoration(
            color: Theme.of(context)
                .colorScheme
                .surfaceContainerHighest
                .withValues(alpha: 0.6),
            borderRadius: BorderRadius.circular(20),
          ),
          child: Text(
            '第 $page / $totalPages 页',
            style: const TextStyle(
                fontSize: 13, fontWeight: FontWeight.w600),
          ),
        ),
        const Spacer(),
        OutlinedButton.icon(
          onPressed: loading ? null : onNext,
          icon: const Icon(Icons.chevron_right, size: 18),
          label: const Text('下一页'),
        ),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// SectionLabel — 带左侧色条的分区标签（表单分节用）
// ════════════════════════════════════════════════════════════════════════════

class SectionLabel extends StatelessWidget {
  const SectionLabel({super.key, required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    final Color primary = Theme.of(context).colorScheme.primary;
    return Row(
      children: <Widget>[
        Container(
          width: 3,
          height: 14,
          decoration: BoxDecoration(
            color: primary,
            borderRadius: BorderRadius.circular(2),
          ),
        ),
        const SizedBox(width: 6),
        Text(
          label,
          style: TextStyle(
            fontWeight: FontWeight.w700,
            fontSize: 13,
            color: primary,
          ),
        ),
      ],
    );
  }
}
