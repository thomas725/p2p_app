import 'package:flutter/material.dart';

/// Bottom navigation shown on full-screen routes (peer info, DM chat) so the
/// user can jump back to any main tab without using the back button.
class MainTabNavBar extends StatelessWidget {
  const MainTabNavBar({
    super.key,
    required this.currentIndex,
    required this.onSelect,
  });
  final int currentIndex;
  final void Function(int) onSelect;

  @override
  Widget build(BuildContext context) {
    return NavigationBar(
      selectedIndex: currentIndex,
      onDestinationSelected: onSelect,
      destinations: const [
        NavigationDestination(icon: Icon(Icons.chat), label: 'Chat'),
        NavigationDestination(icon: Icon(Icons.people), label: 'Peers'),
        NavigationDestination(icon: Icon(Icons.list), label: 'Log'),
        NavigationDestination(icon: Icon(Icons.settings), label: 'Settings'),
      ],
    );
  }
}
