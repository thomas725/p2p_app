import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

/// Groups tab: the list of joined group chats plus a create/join button.
/// Tapping a group opens its full-screen chat via [onOpenGroup]; creating a
/// group goes through the Rust `createGroup` call bound in [onCreateGroup].
class GroupList extends StatefulWidget {
  const GroupList({
    super.key,
    required this.groups,
    required this.onOpenGroup,
    required this.onCreateGroup,
  });
  final List<MobileGroup> groups;
  final void Function(MobileGroup) onOpenGroup;
  final Future<void> Function(String) onCreateGroup;

  @override
  State<GroupList> createState() => _GroupListState();
}

class _GroupListState extends State<GroupList> {
  Future<void> _openCreateDialog() async {
    final created = await showDialog<String>(
      context: context,
      builder: (_) => _CreateGroupDialog(onCreateGroup: widget.onCreateGroup),
    );
    if (created != null && mounted) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('Group "$created" ready.')));
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          color: cs.primaryContainer,
          child: Row(
            children: [
              const Icon(Icons.groups, size: 20),
              const SizedBox(width: 8),
              Text(
                'Groups (${widget.groups.length})',
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(color: cs.onPrimaryContainer),
              ),
              const Spacer(),
              IconButton(
                icon: const Icon(Icons.add),
                tooltip: 'Create or join a group',
                onPressed: _openCreateDialog,
              ),
            ],
          ),
        ),
        Expanded(
          child: widget.groups.isEmpty
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        Icons.groups_outlined,
                        size: 48,
                        color: Colors.grey[400],
                      ),
                      const SizedBox(height: 8),
                      Text(
                        'No groups yet. Create or join one!',
                        style: TextStyle(color: Colors.grey[500]),
                      ),
                    ],
                  ),
                )
              : ListView.separated(
                  itemCount: widget.groups.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (context, i) {
                    final g = widget.groups[i];
                    final members = g.memberCount.toInt();
                    return ListTile(
                      leading: const CircleAvatar(child: Icon(Icons.groups)),
                      title: Text(g.displayName),
                      subtitle: Text(
                        members == 1 ? '1 member' : '$members members',
                      ),
                      trailing: const Icon(Icons.chevron_right),
                      onTap: () => widget.onOpenGroup(g),
                    );
                  },
                ),
        ),
      ],
    );
  }
}

/// Modal dialog for creating (or joining) a public group by name. Submitting
/// calls [onCreateGroup] and pops with the group name on success, or shows an
/// error snackbar (and stays open) on failure.
class _CreateGroupDialog extends StatefulWidget {
  const _CreateGroupDialog({required this.onCreateGroup});
  final Future<void> Function(String) onCreateGroup;

  @override
  State<_CreateGroupDialog> createState() => _CreateGroupDialogState();
}

class _CreateGroupDialogState extends State<_CreateGroupDialog> {
  final _controller = TextEditingController();
  bool _submitting = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final name = _controller.text.trim();
    if (name.isEmpty) return;
    setState(() => _submitting = true);
    try {
      await widget.onCreateGroup(name);
      if (mounted) Navigator.of(context).pop(name);
    } catch (e) {
      if (!mounted) return;
      setState(() => _submitting = false);
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('Failed to create group: $e')));
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Create or join a group'),
      content: TextField(
        controller: _controller,
        autofocus: true,
        decoration: const InputDecoration(
          labelText: 'Group name',
          hintText: 'e.g. rust-lang',
        ),
        onSubmitted: (_) => _submitting ? null : _submit(),
        enabled: !_submitting,
      ),
      actions: [
        TextButton(
          onPressed: _submitting ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _submitting ? null : _submit,
          child: _submitting
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Create'),
        ),
      ],
    );
  }
}