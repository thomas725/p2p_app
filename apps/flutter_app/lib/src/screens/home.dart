import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:p2p_app_flutter/src/rust/api.dart';
import 'package:p2p_app_flutter/src/rust/messages.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/dm_chat.dart';
import 'package:p2p_app_flutter/src/screens/group_chat.dart';
import 'package:p2p_app_flutter/src/screens/group_list.dart';
import 'package:p2p_app_flutter/src/screens/log_tab.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/screens/peer_info.dart';
import 'package:p2p_app_flutter/src/screens/peer_list.dart';
import 'package:p2p_app_flutter/src/screens/settings.dart';
import 'package:p2p_app_flutter/src/util/env.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';

/// Root tabbed screen: Broadcast chat, Peers, Groups, Log, and Settings.
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
  Map<String, PeerMessageStats> _peerStats = {};
  List<MobileGroup> _groups = [];
  int _connectedCount = 0;
  final List<String> _connectedPeerIds = [];
  final List<String> _listenAddresses = [];
  DateTime? _lastConnectionAt;

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
    super.dispose();
  }

  void _onScroll() {
    if (!_scrollController.hasClients) return;
    final pos = _scrollController.position;
    // Use Rust's is_at_bottom logic for consistency
    final atBottom = isAtBottom(
      scrollOffset: pos.pixels.round(),
      total: pos.maxScrollExtent.round(),
      visible: 80,
    );
    if (atBottom != _atBottom) {
      setState(() {
        _atBottom = atBottom;
        if (_atBottom) _unreadCount = 0;
      });
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
    final buf = StringBuffer();
    final sorted = _selectedIndices.toList()..sort();
    for (var i = 0; i < sorted.length; i++) {
      final msg = _messages[sorted[i]];
      final isOwn = msg.peerId == null;
      final peerName = isOwn
          ? 'Me'
          : (msg.senderNickname ??
                (msg.peerId!.length >= 12
                    ? msg.peerId!.substring(0, 12)
                    : msg.peerId!));
      final time = msg.sentAt ?? formatTimeHhmm(dt: msg.createdAt);
      buf.writeln(peerName);
      buf.writeln(msg.content);
      buf.writeln(time);
      if (i < sorted.length - 1) buf.writeln();
    }
    Clipboard.setData(ClipboardData(text: buf.toString()));
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          '${_selectedIndices.length} message${_selectedIndices.length > 1 ? 's' : ''} copied',
        ),
        duration: const Duration(seconds: 2),
      ),
    );
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
      await _refreshPeers();
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

  Future<void> _refreshPeers() async {
    try {
      final rows = await getPeersWithStats();
      final peers = <MobilePeerRecord>[];
      final stats = <String, PeerMessageStats>{};
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
        stats[row.peerId] = PeerMessageStats(
          dmCount: row.dmCount,
          broadcastSentToPeer: row.broadcastSentToPeer,
        );
      }
      setState(() {
        _peers = peers;
        _peerStats = stats;
      });
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

  void _handleSwarmEvent(SwarmEventJson event) {
    if (!mounted) return;
    switch (event.eventType) {
      case 'broadcast':
      case 'dm':
        if (event.content != null && event.peerId != null) {
          _saveIncoming(
            event.content!,
            event.peerId!,
            event.eventType == 'dm',
            event.nickname,
          );
        }
        // A nickname in the event means we just learned or updated this peer's
        // name. Refresh the peer list so the info page (and peer list) reflect
        // the new name instead of the previously id-only/petname display.
        if (event.nickname != null) {
          _refreshPeers();
        }
        break;
      case 'group_message':
        // Incoming group messages are persisted by the Rust swarm handler; a
        // new message means the group's member set may have changed, so refresh
        // the list (counts stay in sync even when no group chat is open).
        _refreshGroups();
        break;
      case 'peer_connected':
        setState(() {
          if (event.peerId != null &&
              !_connectedPeerIds.contains(event.peerId!)) {
            _connectedPeerIds.add(event.peerId!);
          }
          _connectedCount = _connectedPeerIds.length;
        });
        _refreshPeers();
        break;
      case 'peer_discovered':
        _refreshPeers();
        break;
      case 'peer_disconnected':
        setState(() {
          if (event.peerId != null) {
            _connectedPeerIds.remove(event.peerId!);
          }
          _connectedCount = _connectedPeerIds.length;
          _lastConnectionAt = DateTime.now();
        });
        _refreshPeers();
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

  Future<void> _saveIncoming(
    String content,
    String peerId,
    bool isDirect,
    String? nickname,
  ) async {
    try {
      final msg = await saveIncomingMessage(
        content: content,
        peerId: peerId,
        isDirect: isDirect,
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
        startEventPolling();
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
        await _loadHistory();
        await _refreshPeers();
        await _refreshGroups();
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

  void _openPeerInfo(MobilePeerRecord peer) {
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => PeerInfoScreen(
          peer: peer,
          serviceRunning: _serviceRunning,
          onNavigate: _navigateToTab,
          currentTab: 1,
        ),
      ),
    );
  }

  Future<void> _openDmChat(MobilePeerRecord peer) async {
    await Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => DmChatScreen(
          peer: peer,
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
      PeerList(
        peers: _peers,
        stats: _peerStats,
        onOpenInfo: _openPeerInfo,
        onOpenDm: _openDmChat,
        serviceRunning: _serviceRunning,
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
