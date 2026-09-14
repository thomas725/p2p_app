import 'package:flutter/material.dart';

/// Main-tab destinations, shared by the home [NavigationBar] and the
/// [MainTabNavBar] on full-screen routes so both stay in the same order and
/// count: Chat, Peers, Groups, Log, Settings (mirrors the TUI tab layout).
const List<NavigationDestination> kMainTabDestinations = [
  NavigationDestination(icon: Icon(Icons.chat), label: 'Chat'),
  NavigationDestination(icon: Icon(Icons.people), label: 'Peers'),
  NavigationDestination(icon: Icon(Icons.groups), label: 'Groups'),
  NavigationDestination(icon: Icon(Icons.list), label: 'Log'),
  NavigationDestination(icon: Icon(Icons.settings), label: 'Settings'),
];

/// Bottom navigation shown on full-screen routes (peer info, DM chat, group
/// chat) so the user can jump back to any main tab without using the back
/// button.
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
      destinations: kMainTabDestinations,
    );
  }
}
