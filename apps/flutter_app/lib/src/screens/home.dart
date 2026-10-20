import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:p2p_app_flutter/src/data/chat_message_ext.dart';
import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/contact_list.dart';
import 'package:p2p_app_flutter/src/screens/call_screen.dart';
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
import 'package:p2p_app_flutter/src/util/attachments.dart';

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

  /// call_id of the call currently presented by an open [CallScreen] (null when
  /// none). One session at a time: guards against stacking a second call screen
  /// and against placing an outgoing call while another session is live.
  String? _activeCallId;

  /// This install's user identity for the Settings page: our user ID and the
  /// devices bound into it. Refreshed on Settings-tab open and whenever a
  /// claim exchange changes the local user (accepting an intro binds a foreign
  /// device into our user; a `device_binding_received` binds us into a foreign
  /// user).
  String? _ownUserId;
  List<MobileDeviceInfo> _ownDevices = const [];

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
      final status = await nodeApi.getMobilePeerStatus();
      if (mounted) {
        _status = status;
        _syncStatus();
        _syncLiveStatus();
      }
    } catch (_) {}
    await _refreshIdentity();
  }

  /// Resolve our user ID + bound devices through the Rust bindings (the single
  /// source of truth for the identity layer) and surface them on Settings.
  Future<void> _refreshIdentity() async {
    try {
      final userId = await nodeApi.getMyUserId();
      List<MobileDeviceInfo> devices = const [];
      if (userId != null) {
        devices = await nodeApi.getUserDevices(userId: userId);
      }
      if (!mounted) return;
      setState(() {
        _ownUserId = userId;
        _ownDevices = devices;
      });
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

  Future<void> _onScroll() async {
    if (!_scrollController.hasClients) return;
    final atBottom = await chatScrollAtBottom(_scrollController.position);
    if (!mounted || atBottom == _atBottom) return;
    setState(() {
      _atBottom = atBottom;
      if (_atBottom) _unreadCount = 0;
    });
    // The user has actually read up to the newest broadcast: persist the
    // viewed-count watermark so the next launch's unread-skip is right even
    // when they got here by natural scrolling (not the jump-to-bottom button).
    if (atBottom) _saveViewedCount();
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

  Future<void> _copySelected() async {
    await copySelectedMessages(context, _messages, _selectedIndices);
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
        await nodeApi.initMobileDatabase(dbPath: defaultDbPath);
      }
      final status = await nodeApi.getMobilePeerStatus();
      setState(() {
        // The web client is always attached to a server-owned node (its
        // lifecycle belongs to `p2p_chat_server`), so never auto-start one.
        _serviceRunning = running || nodeApi.isRemoteTransport;
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
      } else if (nodeApi.isRemoteTransport) {
        // Events are server-pushed on the remote transport; registering the
        // sink is all that's needed.
        startEventPolling();
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
      final messages = await nodeApi.loadBroadcastMessages(limit: 200);
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
      final rows = await nodeApi.getPeersWithStats();
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
      final contacts = await nodeApi.getContactsWithStats();
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
      final groups = await nodeApi.listGroups();
      setState(() => _groups = groups);
    } catch (_) {}
  }

  Future<void> _createGroup(String name) async {
    await nodeApi.createGroup(name: name);
    await _refreshGroups();
  }

  /// Leave a group: delete its local rows + unsubscribe, then refresh the list.
  /// Leaving is local-only — other members keep the history. A confirmation
  /// dialog lives in [GroupList].
  Future<void> _leaveGroup(MobileGroup group) async {
    await nodeApi.deleteGroup(groupId: group.groupId);
    await _refreshGroups();
  }

  /// Persist a per-peer local nickname (this install only), then refresh the
  /// roster. Returns the refreshed record so an open PeerInfo page reflects the
  /// edit immediately; null when the peer is no longer in the roster.
  Future<MobilePeerRecord?> _setPeerNickname(
    MobilePeerRecord peer,
    String nickname,
  ) async {
    await nodeApi.setPeerLocalNickname(
      peerId: peer.peerId,
      nickname: nickname,
    );
    await _refreshRoster();
    return _peers.where((p) => p.peerId == peer.peerId).firstOrNull;
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
          await nodeApi.acceptDeviceBinding(peerId: id, devicePubkey: pubkey);
          _refreshRoster();
          _refreshIdentity();
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
      final group = await nodeApi.acceptGroupInvite(groupId: groupId);
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
        _refreshIdentity();
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
      case 'receipt':
        // A recipient acknowledged one of our messages: light up its delivery
        // tick in the broadcast chat (DMs are owned by DmChatScreen, which also
        // watches receipts; history reloads read the flag from the DB).
        if (event.msgId != null) {
          _markConfirmed(event.msgId!);
        }
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
      case 'call_invite':
        // An incoming call ring: open the call screen (or implicitly decline
        // while another call is already active — one session at a time).
        if (event.peerId != null && event.content != null) {
          _handleCallInvite(event.peerId!, event.content!);
        }
        break;
    }
    _syncLiveStatus();
  }

  /// Flip the delivery tick on the broadcast message a `receipt` acknowledged.
  /// A no-op for unknown ids (e.g. a DM's receipt, or a message not loaded here).
  void _markConfirmed(String msgId) {
    final idx = _messages.indexWhere((m) => m.msgId == msgId);
    if (idx < 0 || _messages[idx].confirmed) return;
    setState(() => _messages[idx] = _messages[idx].withConfirmed(true));
  }

  Future<void> _handleCallInvite(String peerId, String callId) async {
    if (_activeCallId != null) {
      // We're already in a session; decline the new ring so its caller stops
      // ringing us. Best-effort — the caller times out the invite anyway.
      try {
        await nodeApi.sendCallSignal(peerId: peerId, callId: callId, kind: 'reject');
      } catch (e) {
        debugPrint('Failed to decline concurrent call: $e');
      }
      return;
    }
    final peer = _peers.where((p) => p.peerId == peerId).firstOrNull;
    final displayName = peer?.displayName ?? peerId;
    if (!mounted) return;
    _activeCallId = callId;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => CallScreen(
          peerId: peerId,
          displayName: displayName,
          callId: callId,
          initialPhase: CallScreenPhase.ringing,
          onNavigate: _navigateToTab,
          currentTab: 0,
        ),
      ),
    );
    // Back from the call; drop the live-session guard (the CallScreen has
    // already restored the event sink on dispose).
    if (mounted) setState(() => _activeCallId = null);
  }

  Future<void> _saveIncomingBroadcast(
    String content,
    String peerId,
    String? nickname,
  ) async {
    try {
      final msg = await nodeApi.saveIncomingMessage(
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
      await nodeApi.saveIncomingMessage(
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
      final msg = await nodeApi.saveOutgoingBroadcast(content: text);
      setState(() => _messages.add(msg));
      _forceScrollToBottom();
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  /// Pick a file and send it as an attachment-only broadcast (files lane §12).
  /// Native picks send by path; web picks upload the picked bytes.
  Future<void> _pickAndSendBroadcast() async {
    final picked = await pickFileOverride();
    if (picked == null) return;
    try {
      final msg = await nodeApi.saveOutgoingBroadcast(
        content: '',
        attachmentPath: picked.path,
        attachmentBytes: picked.bytes,
        attachmentName: picked.name,
      );
      setState(() => _messages.add(msg));
      _forceScrollToBottom();
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  /// Resolve and open a received attachment's stored blob.
  Future<void> _openBroadcastAttachment(ChatMessage msg) async {
    final error = await openAttachedFile(msg);
    if (!mounted) return;
    ScaffoldMessenger.of(context)
      ..clearSnackBars()
      ..showSnackBar(
        SnackBar(
          content: Text(error ?? 'Opening ${msg.attachment?.name}'),
          duration: const Duration(seconds: 2),
        ),
      );
  }

  Future<void> _toggleService() async {
    try {
      if (_serviceRunning) {
        if (isAndroid) {
          await serviceChannel.invokeMethod('stopService');
        }
        stopEventPolling();
        await nodeApi.stopNode();
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
          await nodeApi.startNode(dbPath: defaultDbPath);
        } else {
          await nodeApi.startNodeAuto();
        }
        setState(() {
          _serviceRunning = true;
          _connectedCount = 0;
          _connectedPeerIds.clear();
          _listenAddresses.clear();
          _lastConnectionAt = null;
        });
        final status = await nodeApi.getMobilePeerStatus();
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
          onCall: (p) => _startCallTo(p.peerId, p.displayName),
          onEditLocalNickname: _setPeerNickname,
        ),
      ),
    );
  }

  /// Place an outgoing call to a contact device: mint a fresh session id via
  /// the Rust `start_call` binding and open the call screen in its `calling`
  /// phase. A no-op while a call is already active (one session at a time).
  Future<void> _startCall(MobileContactDevice device) async {
    await _startCallTo(device.peerId, device.displayName);
  }

  Future<void> _startCallTo(String peerId, String displayName) async {
    if (_activeCallId != null) return;
    try {
      final callId = await nodeApi.startCall(peerId: peerId);
      if (!mounted) return;
      _activeCallId = callId;
      await Navigator.of(context).push(
        MaterialPageRoute<void>(
          builder: (_) => CallScreen(
            peerId: peerId,
            displayName: displayName,
            callId: callId,
            initialPhase: CallScreenPhase.calling,
            onNavigate: _navigateToTab,
            currentTab: 1,
          ),
        ),
      );
      if (mounted) setState(() => _activeCallId = null);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to start call: $e')),
        );
      }
    }
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

  /// Send our device-binding intro to a device so its account holder can bind
  /// us into their account (the primary "add my second device" direction: run
  /// this on the new device, targeting the account holder already in our
  /// roster). Confirms via a snackbar; errors surface inline.
  Future<void> _sendIntro(MobileContactDevice device) async {
    final messenger = ScaffoldMessenger.maybeOf(context);
    try {
      await nodeApi.sendDeviceBindingIntro(peerId: device.peerId);
      messenger?.showSnackBar(
        SnackBar(content: Text('Device intro sent to ${device.displayName}')),
      );
    } catch (e) {
      messenger?.showSnackBar(
        SnackBar(content: Text('Failed to send device intro: $e')),
      );
    }
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
          onCall: (p) => _startCallTo(p.peerId, p.displayName),
          onEditLocalNickname: _setPeerNickname,
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
        onPickFile: _pickAndSendBroadcast,
        onOpenAttachment: _openBroadcastAttachment,
      ),
      ContactList(
        rows: _contacts,
        onOpenInfo: _openPeerInfo,
        onOpenDm: _openDmChat,
        onLinkDevice: _sendIntro,
        onCallDevice: _startCall,
      ),
      GroupList(
        groups: _groups,
        onOpenGroup: _openGroupChat,
        onCreateGroup: _createGroup,
        onDeleteGroup: _leaveGroup,
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
        userId: _ownUserId,
        devices: _ownDevices,
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
