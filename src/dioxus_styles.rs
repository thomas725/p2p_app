//! CSS stylesheet for the Dioxus UI.

pub const STYLESHEET: &str = "
* { margin: 0; padding: 0; box-sizing: border-box; }
body { background: #1a1a2e; color: #e0e0e0; font-family: 'Segoe UI', system-ui, sans-serif; overflow: hidden; }
.app { display: flex; flex-direction: column; height: 100vh; }
.header { display: flex; align-items: center; gap: 16px; padding: 8px 16px; background: #16213e; border-bottom: 1px solid #0f3460; }
.header h1 { font-size: 18px; color: #e94560; margin: 0; }
.header .status { font-size: 12px; padding: 2px 8px; border-radius: 10px; }
.header .online { background: #1b5e20; color: #a5d6a7; }
.header .offline { background: #b71c1c; color: #ef9a9a; }
.header .peer-count { font-size: 12px; color: #90caf9; }
.header .nickname { font-size: 12px; color: #ce93d8; }
.header .peer-id { font-size: 11px; color: #78909c; }
.tab-bar { display: flex; background: #16213e; border-bottom: 1px solid #0f3460; overflow-x: auto; }
.tab { padding: 8px 16px; background: none; border: none; color: #90caf9; cursor: pointer; font-size: 13px; white-space: nowrap; border-bottom: 2px solid transparent; }
.tab:hover { background: #1a1a3e; }
.tab.active { color: #e94560; border-bottom-color: #e94560; }
.content { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.messages { flex: 1; overflow-y: auto; padding: 8px 16px; }
.message { padding: 6px 8px; border-bottom: 1px solid #0f3460; font-size: 14px; line-height: 1.4; }
.message:hover { background: #16213e; }
.receipt-info { font-size: 11px; color: #66bb6a; margin-left: 8px; }
.message-input { display: flex; padding: 8px 16px; background: #16213e; border-top: 1px solid #0f3460; gap: 8px; }
.input-field { flex: 1; padding: 8px 12px; background: #0f3460; border: 1px solid #1a1a4e; border-radius: 6px; color: #e0e0e0; font-size: 14px; outline: none; }
.input-field:focus { border-color: #e94560; }
.send-btn { padding: 8px 20px; background: #e94560; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 14px; }
.send-btn:hover { background: #c73650; }
.peers-view { flex: 1; padding: 16px; overflow-y: auto; }
.peers-view h2 { font-size: 16px; color: #90caf9; margin-bottom: 12px; }
.peer-list { display: flex; flex-direction: column; gap: 2px; }
.peer-item { display: flex; align-items: center; padding: 8px 12px; background: #16213e; border-radius: 4px; font-size: 13px; gap: 16px; }
.peer-header { color: #78909c; font-weight: bold; font-size: 12px; text-transform: uppercase; }
.peer-item span { flex: 1; }
.peer-item .peer-id { font-family: monospace; color: #90caf9; }
.peer-item .peer-nickname { color: #ce93d8; }
.peer-item .peer-actions { flex: 0; }
.peer-item button, .modal-buttons button { padding: 4px 12px; background: #0f3460; color: #90caf9; border: 1px solid #1a1a4e; border-radius: 4px; cursor: pointer; font-size: 12px; }
.peer-item button:hover, .modal-buttons button:hover { background: #1a1a4e; }
.log-view { flex: 1; overflow-y: auto; padding: 8px 16px; }
.log-entry { padding: 2px 0; font-size: 12px; color: #78909c; font-family: monospace; }
.modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 100; }
.modal { background: #16213e; border: 1px solid #0f3460; border-radius: 8px; padding: 24px; min-width: 320px; max-width: 500px; }
.modal h3 { font-size: 16px; color: #e0e0e0; margin-bottom: 16px; }
.modal .input-field { width: 100%; margin-bottom: 16px; }
.modal-buttons { display: flex; gap: 8px; justify-content: flex-end; }
.modal pre { white-space: pre-wrap; font-size: 13px; color: #e0e0e0; margin-bottom: 16px; max-height: 300px; overflow-y: auto; }
.dm-messages h3 { font-size: 14px; color: #90caf9; padding: 8px 16px; border-bottom: 1px solid #0f3460; }
";
