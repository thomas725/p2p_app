import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:p2p_app_flutter/src/rust/api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/contact_list.dart';
import 'package:p2p_app_flutter/src/screens/dm_chat.dart';
import 'package:p2p_app_flutter/src/screens/group_chat.dart';
import 'package:p2p_app_flutter/src/screens/group_list.dart';
import 'package:p2p_app_flutter/src/screens/log_tab.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/screens/peer_info.dart';
import 'package:p2p_app_flutter/src/screens/settings.dart';
import 'package:p2p_app_flutter/src/util/ui_utils.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';

/// Prompt whether to bind an incoming device into this account. Shown when a
/// peer's `device_binding_intro` arrives; accepting mints a signed claim that
/// adds that peer's device to our user's device-set.
///
/// Test-only seam: [accept] replaces the FFI-backed `acceptDeviceBinding` call
/// so widget tests can exercise the flow without a loaded Rust library.
@visibleForTesting
Future<void> promptDeviceBinding(
  BuildContext context, {
  required String peerId,
  required Uint8List devicePubkey,
  required Future<void> Function(String peerId, Uint8List devicePubkey) accept,
}) async {
  final allowed = await showDialog<bool>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: const Text('Link device to my account?'),
      content: Text('Allow $peerId to join your account as a device?'),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(ctx, false),
          child: const Text('Decline'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(ctx, true),
          child: const Text('Allow'),
        ),
      ],
    ),
  );
  if (allowed == true) {
    await accept(peerId, devicePubkey);
  }
}

/// Root tabbed screen: Broadcast chat, Contacts, Groups, Log, and Settings.
class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  int _tabIndex = 0;
  MobilePeerStatus? _status;
  String? _error;
  bool _loading = true;
  bool _serviceRunning = false;
  bool _atBottom = true;
  int _unreadCount = 0;
  bool _selectionMode = false;
  final Set<int> _selectedIndices = {};

  final List<ChatMessage> _messages = [];
  List<MobilePeerRecord> _peers = [];
  List<MobileContactRow> _contacts = [];
  List<MobileGroup> _groups = [];
  int _connectedCount = 0;
  final List<String> _connectedPeerIds = [];
  final List<String> _listenAddresses = [];
  DateTime? _lastConnectionAt;

  /// Device-binding intros already being prompted for (peer_id → device_pubkey),
  /// so repeated `device_binding_intro` events never stack dialogs.
  final Map<String, Uint8List> _pendingDeviceBindings = {};

  /// Live connection snapshot, broadcast to the Settings page.
  final ValueNotifier<LiveStatus> _liveStatus = ValueNotifier(
    const LiveStatus(
      connectedCount: 0,
      connectedPeerIds: [],
      listenAddresses: [],
      lastConnectionAt: null,
    ),
  );

  /// Live node status (peer id, nickname, db url), broadcast to the Settings page.
  final ValueNotifier<MobilePeerStatus?> _statusNotifier =
      ValueNotifier<MobilePeerStatus?>(null);

  void _syncLiveStatus() {
    _liveStatus.value = LiveStatus(
      connectedCount: _connectedCount,
      connectedPeerIds: List<String>.from(_connectedPeerIds),
      listenAddresses: List<String>.from(_listenAddresses),
      lastConnectionAt: _lastConnectionAt,
    );
  }

  void _syncStatus() {
    _statusNotifier.value = _status;
  }

  /// Re-fetch the node status when the Settings tab is (re)opened so it always
  /// shows current data even if it was offscreen when something changed.
  Future<void> _refreshSettingsOnOpen() async {
    if (!_serviceRunning) return;
    try {
      final status = await getMobilePeerStatus();
      if (mounted) {
        _status = status;
        _syncStatus();
        _syncLiveStatus();
      }
    } catch (_) {}
  }

  @override
  void initState() {
    super.initState();
    setEventSink(_handleSwarmEvent);
    _scrollController.addListener(_onScroll);
    _init();
  }

  @override
  void dispose() {
    _scrollController.removeListener(_onScroll);
    setEventSink(null);
    _scrollController.dispose();
    _liveStatus.dispose();
    _statusNotifier.dispose();
    super.dispose();
  }

  void _onScroll() {
    if (!_scrollController.hasClients) return;
    final atBottom = chatScrollAtBottom(_scrollController.position);
    if (atBottom != _atBottom) {
      setState(() {
        _atBottom = atBottom;
        if (_atBottom) _unreadCount = 0;
      });
      // The user has actually read up to the newest broadcast: persist the
      // viewed-count watermark so the next launch's unread-skip is right even
      // when they got here by natural scrolling (not the jump-to-bottom button).
      if (atBottom) _saveViewedCount();
    }
  }

  void _onBubbleDoubleTap(int index) {
    setState(() {
      _selectionMode = true;
      _selectedIndices.add(index);
    });
  }

  void _onBubbleTap(int index) {
    if (!_selectionMode) return;
    setState(() {
      if (_selectedIndices.contains(index)) {
        _selectedIndices.remove(index);
        if (_selectedIndices.isEmpty) _selectionMode = false;
      } else {
        _selectedIndices.add(index);
      }
    });
  }

  void _cancelSelection() {
    setState(() {
      _selectionMode = false;
      _selectedIndices.clear();
    });
  }

  void _copySelected() {
    copySelectedMessages(context, _messages, _selectedIndices);
    _cancelSelection();
  }

  Future<void> _init() async {
    try {
      bool running = false;
      if (isAndroid) {
        running =
            await serviceChannel.invokeMethod<bool>('isServiceRunning') ??
            false;
      }
      if (isAndroid) {
        await initMobileDatabase(dbPath: defaultDbPath);
      }
      final status = await getMobilePeerStatus();
      setState(() {
        _serviceRunning = running;
        _status = status;
        _loading = false;
      });
      _syncStatus();
      _syncLiveStatus();
      await _loadHistory();
      await _refreshRoster();
      await _refreshGroups();
      await _scrollToFirstUnread();
      // Auto-start the node (on Android this also starts the foreground service)
      if (!_serviceRunning) {
        await _toggleService();
      }
    } catch (e) {
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  Future<void> _loadHistory() async {
    try {
      final messages = await loadBroadcastMessages(limit: 200);
      setState(() {
        _messages
          ..clear()
          ..addAll(messages);
      });
    } catch (_) {}
  }

  /// Refresh the whole roster in one round trip: the peer list + stats (used by
  /// Settings and broadcast-sender resolution) and the aggregated Contacts-tab
  /// rows (single round trip via `getContactsWithStats`).
  Future<void> _refreshRoster() async {
    try {
      final rows = await getPeersWithStats();
      final peers = <MobilePeerRecord>[];
      for (final row in rows) {
        peers.add(
          MobilePeerRecord(
            peerId: row.peerId,
            firstSeen: row.firstSeen,
            lastSeen: row.lastSeen,
            nickname: row.nickname,
            localNickname: row.localNickname,
            displayName: row.displayName,
          ),
        );
      }
      final contacts = await getContactsWithStats();
      if (mounted) {
        setState(() {
          _peers = peers;
          _contacts = contacts;
        });
      }
    } catch (_) {}
  }

  Future<void> _refreshGroups() async {
    try {
      final groups = await listGroups();
      setState(() => _groups = groups);
    } catch (_) {}
  }

  Future<void> _createGroup(String name) async {
    await createGroup(name: name);
    await _refreshGroups();
  }

  /// Prompt to bind an incoming device into this account. Deduplicates by
  /// peer_id so retransmitted intros never stack dialogs.
  Future<void> _handleDeviceBindingIntro(
    String peerId,
    Uint8List devicePubkey,
  ) async {
    if (_pendingDeviceBindings.containsKey(peerId)) return;
    _pendingDeviceBindings[peerId] = devicePubkey;
    try {
      await promptDeviceBinding(
        context,
        peerId: peerId,
        devicePubkey: devicePubkey,
        accept: (id, pubkey) async {
          await acceptDeviceBinding(peerId: id, devicePubkey: pubkey);
          _refreshRoster();
        },
      );
    } finally {
      _pendingDeviceBindings.remove(peerId);
    }
  }

  /// Accept an incoming group invite: persist membership + subscribe (like the
  /// TUI) and refresh the Groups list so the accepted group appears.
  Future<void> _acceptInvite(String groupId) async {
    try {
      final group = await acceptGroupInvite(groupId: groupId);
      if (mounted) {
        setState(() {
          if (!_groups.any((g) => g.groupId == group.groupId)) {
            _groups.add(group);
          }
        });
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Joined group: ${group.displayName}'),
            duration: const Duration(seconds: 2),
          ),
        );
      }
    } catch (e) {
      debugPrint('Failed to accept group invite: $e');
    } finally {
      _refreshGroups();
    }
  }

  void _handleSwarmEvent(SwarmEventJson event) {
    if (!mounted) return;
    switch (event.eventType) {
      case 'broadcast':
        if (event.content != null && event.peerId != null) {
          _saveIncomingBroadcast(event.content!, event.peerId!, event.nickname);
        }
        // A nickname in the event means we just learned or updated this peer's
        // name. Refresh the peer list so the info page (and peer list) reflect
        // the new name instead of the previously id-only/petname display.
        if (event.nickname != null) {
          _refreshRoster();
        }
        break;
      case 'dm':
        // Persist incoming DMs for history. They must never be appended to the
        // broadcast chat; an open DmChatScreen owns the event sink and persists
        // its own messages, so this path only runs while no DM screen is open.
        if (event.content != null && event.peerId != null) {
          _persistIncomingDm(event.content!, event.peerId!, event.nickname);
        }
        if (event.nickname != null) {
          _refreshRoster();
        }
        break;
      case 'group_message':
        // Incoming group messages are persisted by the Rust swarm handler; a
        // new message means the group's member set may have changed, so refresh
        // the list (counts stay in sync even when no group chat is open).
        _refreshGroups();
        break;
      case 'group_invite':
        // Accept incoming group invites (record local membership + subscribe),
        // mirroring the TUI's auto-accept behavior, then surface the group.
        if (event.groupId != null) {
          _acceptInvite(event.groupId!);
        }
        break;
      case 'device_binding_intro':
        // A peer advertises its device for binding into our account. Prompt
        // the account holder; only an explicit Allow mints the claim.
        if (event.peerId != null && event.devicePubkey != null) {
          _handleDeviceBindingIntro(event.peerId!, event.devicePubkey!);
        }
        break;
      case 'device_binding_received':
        // Our account gained a device (e.g. we accepted a peer's intro), so
        // the roster's user-grouping may have changed.
        _refreshRoster();
        break;
      case 'peer_connected':
        setState(() {
          if (event.peerId != null &&
              !_connectedPeerIds.contains(event.peerId!)) {
            _connectedPeerIds.add(event.peerId!);
          }
          _connectedCount = _connectedPeerIds.length;
        });
        _refreshRoster();
        break;
      case 'peer_discovered':
        _refreshRoster();
        break;
      case 'peer_disconnected':
        setState(() {
          if (event.peerId != null) {
            _connectedPeerIds.remove(event.peerId!);
          }
          _connectedCount = _connectedPeerIds.length;
          _lastConnectionAt = DateTime.now();
        });
        _refreshRoster();
        break;
      case 'listen_addr':
        if (event.address != null &&
            !_listenAddresses.contains(event.address)) {
          setState(() => _listenAddresses.add(event.address!));
        }
        break;
    }
    _syncLiveStatus();
  }

  Future<void> _saveIncomingBroadcast(
    String content,
    String peerId,
    String? nickname,
  ) async {
    try {
      final msg = await saveIncomingMessage(
        content: content,
        peerId: peerId,
        isDirect: false,
        nickname: nickname,
      );
      if (mounted) {
        setState(() {
          _messages.add(msg);
          if (!_atBottom) _unreadCount++;
        });
        _scrollToBottom();
      }
    } catch (e) {
      debugPrint('Failed to save incoming: $e');
    }
  }

  /// Persist an incoming DM without displaying it in the broadcast chat.
  Future<void> _persistIncomingDm(
    String content,
    String peerId,
    String? nickname,
  ) async {
    try {
      await saveIncomingMessage(
        content: content,
        peerId: peerId,
        isDirect: true,
        nickname: nickname,
      );
    } catch (e) {
      debugPrint('Failed to save incoming DM: $e');
    }
  }

  final _scrollController = ScrollController();

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients && _atBottom) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      }
    });
  }

  void _forceScrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
        setState(() {
          _atBottom = true;
          _unreadCount = 0;
        });
      }
    });
    _saveViewedCount();
  }

  static const _lastViewedKey = 'broadcast_last_viewed_count';

  Future<void> _saveViewedCount() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_lastViewedKey, _messages.length);
  }

  Future<void> _scrollToFirstUnread() async {
    final prefs = await SharedPreferences.getInstance();
    final lastViewed = prefs.getInt(_lastViewedKey) ?? 0;
    final unreadCount = _messages.length - lastViewed;

    if (unreadCount > 0 && _messages.isNotEmpty) {
      _unreadCount = unreadCount;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_scrollController.hasClients) {
          final firstUnreadIndex = lastViewed.clamp(0, _messages.length - 1);
          final targetOffset = (firstUnreadIndex * 70.0) - 20.0;
          _scrollController.jumpTo(
            targetOffset.clamp(0.0, _scrollController.position.maxScrollExtent),
          );
        }
      });
    } else {
      _forceScrollToBottom();
    }
  }

  Future<void> _sendBroadcast(String text) async {
    if (text.isEmpty) return;
    try {
      final msg = await saveOutgoingBroadcast(content: text);
      setState(() => _messages.add(msg));
      _forceScrollToBottom();
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  Future<void> _toggleService() async {
    try {
      if (_serviceRunning) {
        if (isAndroid) {
          await serviceChannel.invokeMethod('stopService');
        }
        stopEventPolling();
        await stopNode();
        setState(() {
          _serviceRunning = false;
          _connectedCount = 0;
          _connectedPeerIds.clear();
          _listenAddresses.clear();
          _groups = [];
        });
        _syncLiveStatus();
      } else {
        if (isAndroid) {
          await serviceChannel.invokeMethod('startService', {
            'dbPath': defaultDbPath,
          });
        }
        if (isAndroid) {
          await startNode(dbPath: defaultDbPath);
        } else {
          await startNodeAuto();
        }
        setState(() {
          _serviceRunning = true;
          _connectedCount = 0;
          _connectedPeerIds.clear();
          _listenAddresses.clear();
          _lastConnectionAt = null;
        });
        final status = await getMobilePeerStatus();
        setState(() => _status = status);
        _syncStatus();
        _syncLiveStatus();
        // Load history/peers BEFORE the poller forwards live events: a message
        // that arrives while polling is already forwarding would otherwise be
        // appended, then wiped out a moment later when this reload `clear()`s
        // the list with a snapshot that predates the new insert.
        await _loadHistory();
        await _refreshRoster();
        await _refreshGroups();
        startEventPolling();
      }
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  Future<void> _shareApk() async {
    if (!isAndroid) return;
    try {
      await serviceChannel.invokeMethod('shareApk');
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  // Switch the main tab from a full-screen route (peer info / DM / group chat)
  // so the user can navigate without the back button.
  void _navigateToTab(int i) {
    setState(() => _tabIndex = i);
    if (i == 4) _refreshSettingsOnOpen();
  }

  /// Materialize a [MobilePeerRecord] from an exact device row (the peer-info
  /// and DM screens take a peer, not a contact).
  MobilePeerRecord _deviceToPeer(MobileContactDevice device) {
    return MobilePeerRecord(
      peerId: device.peerId,
      firstSeen: device.firstSeen,
      lastSeen: device.lastSeen,
      nickname: device.receivedNickname,
      localNickname: device.localNickname,
      displayName: device.displayName,
    );
  }

  /// Contact rows pass their best device ([MobileContactRow.deviceRows] is
  /// sorted by the canonical model, so the first device is the newest); device
  /// sub-rows pass the exact device the row represents.
  void _openPeerInfo(MobileContactRow contact, MobileContactDevice device) {
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => PeerInfoScreen(
          peer: _deviceToPeer(device),
          serviceRunning: _serviceRunning,
          onNavigate: _navigateToTab,
          currentTab: 1,
        ),
      ),
    );
  }

  Future<void> _openDmChat(
    MobileContactRow contact,
    MobileContactDevice device,
  ) async {
    await Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => DmChatScreen(
          peer: _deviceToPeer(device),
          serviceRunning: _serviceRunning,
          onNavigate: _navigateToTab,
          currentTab: 1,
        ),
      ),
    );
    // The DM chat screen takes over the event sink while open; hand it back so
    // home keeps receiving broadcast/DM events after we return.
    if (mounted) setEventSink(_handleSwarmEvent);
  }

  void _openGroupChat(MobileGroup group) {
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => GroupChatScreen(
          group: group,
          serviceRunning: _serviceRunning,
          onNavigate: _navigateToTab,
          currentTab: 2,
        ),
      ),
    ).then((_) {
      // The group chat screen hands the event sink back on pop; also refresh
      // the group list so member counts reflect what was seen while open.
      if (mounted) {
        setEventSink(_handleSwarmEvent);
        _refreshGroups();
      }
    });
  }

  // Open the info page for the sender of a broadcast message. If the peer
  // isn't in the discovered list yet, synthesize a record from what we know.
  void _openPeerInfoForPeerId(String peerId) {
    final known = _peers.where((p) => p.peerId == peerId).firstOrNull;
    final record =
        known ??
        MobilePeerRecord(
          peerId: peerId,
          firstSeen: '',
          lastSeen: '',
          nickname: null,
          localNickname: null,
          displayName: peerId,
        );
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => PeerInfoScreen(
          peer: record,
          serviceRunning: _serviceRunning,
          onNavigate: _navigateToTab,
          currentTab: 0,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    if (_error != null) {
      return Scaffold(
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(_error!, textAlign: TextAlign.center),
              const SizedBox(height: 16),
              FilledButton(onPressed: _init, child: const Text('Retry')),
            ],
          ),
        ),
      );
    }

    final screens = [
      BroadcastChat(
        messages: _messages,
        scrollController: _scrollController,
        serviceRunning: _serviceRunning,
        onSend: _sendBroadcast,
        unreadCount: _unreadCount,
        onJumpToBottom: _forceScrollToBottom,
        selectionMode: _selectionMode,
        selectedIndices: _selectedIndices,
        onBubbleDoubleTap: _onBubbleDoubleTap,
        onBubbleTap: _onBubbleTap,
        onCancelSelection: _cancelSelection,
        onCopySelected: _copySelected,
        onOpenPeerInfo: _openPeerInfoForPeerId,
      ),
      ContactList(
        rows: _contacts,
        onOpenInfo: _openPeerInfo,
        onOpenDm: _openDmChat,
      ),
      GroupList(
        groups: _groups,
        onOpenGroup: _openGroupChat,
        onCreateGroup: _createGroup,
      ),
      const LogTab(),
      Settings(
        liveStatus: _liveStatus,
        status: _statusNotifier,
        serviceRunning: _serviceRunning,
        onToggleService: _toggleService,
        onShareApk: _shareApk,
        isAndroid: isAndroid,
        peers: _peers,
        networkName: kNetworkName,
        onOpenPeerInfo: _openPeerInfoForPeerId,
      ),
    ];

    return Scaffold(
      body: IndexedStack(index: _tabIndex, children: screens),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _tabIndex,
        onDestinationSelected: (i) {
          setState(() => _tabIndex = i);
          if (i == 4) _refreshSettingsOnOpen();
        },
        destinations: kMainTabDestinations,
      ),
    );
  }
}
