# User Stories

## Overview

This document captures user stories and acceptance criteria for the P2P Chat application.

## Core Functionality

### Chat Features

- [x] **US-001**: Broadcast messages to all connected peers via gossipsub
  - **As a** user
  - **I want to** send a message that all connected peers receive
  - **So that** I can communicate with the entire network

- [x] **US-002**: Send direct messages to specific peers
  - **As a** user
  - **I want to** send a private message to a specific peer
  - **So that** I can have 1-on-1 conversations

- [x] **US-003**: View chat history in the Chat tab
  - **As a** user
  - **I want to** see previous broadcast messages
  - **So that** I can catch up on conversations

- [x] **US-004**: View message history with specific peers in DM tabs
  - **As a** user
  - **I want to** see previous direct messages with a peer
  - **So that** I can review private conversations

### Peer Management

- [x] **US-010**: See a list of discovered peers with their connection status
  - **As a** user
  - **I want to** see all peers discovered via mDNS
  - **So that** I know who is on the network

- [x] **US-011**: Set local nicknames for peers
  - **As a** user
  - **I want to** assign friendly names to peer IDs
  - **So that** I can identify peers more easily

- [x] **US-012**: Click on peer to open DM tab
  - **As a** user
  - **I want to** click on a peer in the Peers tab
  - **So that** I can quickly start a direct message

- [x] **US-013**: Auto-discover peers via mDNS
  - **As a** user
  - **I want to** automatically find other chat instances on the local network
  - **So that** I don't need to manually configure peers

### Navigation & UI

- [x] **US-020**: Navigate between tabs using Tab key
  - **As a** user
  - **I want to** press Tab to cycle through tabs
  - **So that** I can quickly switch views

- [x] **US-021**: Click tabs to switch views
  - **As a** user
  - **I want to** click on tab headers
  - **So that** I can switch views with the mouse

- [x] **US-022**: View logs with timestamps
  - **As a** user
  - **I want to** see system logs in the Log tab
  - **So that** I can debug network issues

- [x] **US-023**: Close DM tabs with Ctrl+W
  - **As a** user
  - **I want to** close DM tabs with a keyboard shortcut
  - **So that** I can manage my workspace efficiently

- [x] **US-024**: Go to notification tab with Ctrl+N or Ctrl+G
  - **As a** user
  - **I want to** jump to the notification tab
  - **So that** I can quickly see new messages

### Nickname System

- [x] **US-030**: Set my display name with /nick command
  - **As a** user
  - **I want to** set my nickname using `/nick <name>`
  - **So that** peers see my preferred name

- [x] **US-031**: Set local nickname for peer with /setpeer
  - **As a** user
  - **I want to** set a local alias for a peer
  - **So that** I can identify them by a custom name

## Technical Features

### Developer Stories

- [x] **US-100**: Run in TUI mode
  - **As a** developer
  - **I want to** run with an interactive terminal UI
  - **So that** users have a nice interface

- [x] **US-101**: Run in headless mode
  - **As a** developer
  - **I want to** run without a UI for server/automation
  - **So that** I can integrate with other tools

- [x] **US-102**: Configure network mesh parameters
  - **As a** developer
  - **I want to** tune gossipsub based on expected peer count
  - **So that** the app works well for different network sizes

- [x] **US-103**: Recover from stale peer addresses via mDNS
  - **As a** developer
  - **I want to** automatically rediscover peers after address changes
  - **So that** connectivity is maintained

- [x] **US-104**: Persist data to SQLite
  - **As a** developer
  - **I want to** store messages, peers, and identity in a database
  - **So that** data persists across restarts

## Future User Stories (Not Yet Implemented)

### Chat Enhancements

- [ ] **US-005**: Edit or delete sent messages
- [ ] **US-006**: Reply to specific messages
- [ ] **US-007**: Send file attachments
- [ ] **US-008**: Message reactions/emoji
- [ ] **US-009**: Typing indicators

### UI Enhancements

- [ ] **US-025**: Dark/light theme toggle
- [ ] **US-026**: Customizable color scheme
- [ ] **US-027**: Notification sounds
- [ ] **US-028**: Desktop notifications for new messages
- [ ] **US-029**: Keyboard shortcuts help overlay

### Platform Features

- [ ] **US-040**: Web interface via Dioxus
- [ ] **US-041**: Mobile app via Dioxus
- [ ] **US-042**: End-to-end encryption for DMs
- [ ] **US-043**: File transfer between peers
- [ ] **US-044**: Voice/video calls

### Network Features

- [ ] **US-050**: Connect to public bootstrap nodes
- [ ] **US-051**: LAN-only mode toggle
- [ ] **US-052**: Connection quality indicators
- [ ] **US-053**: Manual peer address entry
- [ ] **US-054**: Peer blocklist/mute

## Test Coverage

### Unit Tests
- Network size classification
- ANSI code stripping
- Timestamp formatting
- TUI layout calculations
- Dynamic tabs management
- Peer extraction from messages

### Integration Tests
- P2P message transfer
- Bidirectional messaging
- Auto-discovery via mDNS
- Stale address recovery
- Direct message protocol
- Message deduplication
- Peer persistence

### UI Tests
- Mouse click handling
- Multi-line message rendering
- Scroll offset calculations
- Tab switching
- Notification display
