// Multiplayer — WebSocket presence, position broadcasting

export interface RemotePlayer {
  pubkey: string;
  displayName: string;
  x: number;
  y: number;
  z: number;
  rotY: number;
  lastUpdate: number;
}

// --- State ---

const remotePlayers = new Map<string, RemotePlayer>();
let ws: WebSocket | null = null;
let connected = false;
let localPubkey = "";
let localDisplayName = "";
let currentServerUrl = "";
let lastSendTime = 0;
let reconnectDelay = 1000;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let staleCleanupTimer: ReturnType<typeof setInterval> | null = null;

// --- Callbacks ---

type JoinCallback = (player: RemotePlayer) => void;
type LeaveCallback = (pubkey: string) => void;
type MoveCallback = (player: RemotePlayer) => void;

const joinCallbacks: JoinCallback[] = [];
const leaveCallbacks: LeaveCallback[] = [];
const moveCallbacks: MoveCallback[] = [];

// --- Public API ---

export function connect(serverUrl: string, pubkey: string, displayName: string): void {
  localPubkey = pubkey;
  localDisplayName = displayName;
  currentServerUrl = serverUrl;
  reconnectDelay = 1000;

  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }

  openSocket();
  startStaleCleanup();
}

export function disconnect(): void {
  if (reconnectTimer !== null) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  stopStaleCleanup();

  if (ws) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "leave" }));
    }
    ws.onclose = null;
    ws.onerror = null;
    ws.onmessage = null;
    ws.onopen = null;
    ws.close();
    ws = null;
  }

  connected = false;
  remotePlayers.clear();
}

export function sendPosition(x: number, y: number, z: number, rotY: number): void {
  if (!ws || ws.readyState !== WebSocket.OPEN) return;

  const now = performance.now();
  if (now - lastSendTime < 66) return; // ~15 Hz throttle
  lastSendTime = now;

  ws.send(JSON.stringify({ type: "position", x, y, z, rotY }));
}

export function getRemotePlayers(): Map<string, RemotePlayer> {
  return remotePlayers;
}

export function onPlayerJoin(callback: JoinCallback): void {
  joinCallbacks.push(callback);
}

export function onPlayerLeave(callback: LeaveCallback): void {
  leaveCallbacks.push(callback);
}

export function onPlayerMove(callback: MoveCallback): void {
  moveCallbacks.push(callback);
}

export function isConnected(): boolean {
  return connected;
}

export function getPlayerCount(): number {
  return remotePlayers.size;
}

// --- Internal ---

function openSocket(): void {
  if (ws) {
    ws.onclose = null;
    ws.onerror = null;
    ws.onmessage = null;
    ws.onopen = null;
    ws.close();
    ws = null;
  }

  ws = new WebSocket(currentServerUrl);

  ws.onopen = () => {
    connected = true;
    reconnectDelay = 1000;
    ws!.send(
      JSON.stringify({
        type: "join",
        pubkey: localPubkey,
        displayName: localDisplayName,
      }),
    );
  };

  ws.onclose = () => {
    connected = false;
    scheduleReconnect();
  };

  ws.onerror = () => {
    // onclose will fire after onerror, reconnect handled there
  };

  ws.onmessage = (event) => {
    handleMessage(event.data);
  };
}

function handleMessage(raw: unknown): void {
  try {
    const msg = JSON.parse(raw as string);

    switch (msg.type) {
      case "player_join": {
        const player: RemotePlayer = {
          pubkey: msg.pubkey,
          displayName: msg.displayName,
          x: 0,
          y: 0,
          z: 0,
          rotY: 0,
          lastUpdate: Date.now(),
        };
        remotePlayers.set(msg.pubkey, player);
        for (const cb of joinCallbacks) cb(player);
        break;
      }

      case "player_leave": {
        remotePlayers.delete(msg.pubkey);
        for (const cb of leaveCallbacks) cb(msg.pubkey);
        break;
      }

      case "player_position": {
        let player = remotePlayers.get(msg.pubkey);
        if (player) {
          player.x = msg.x;
          player.y = msg.y;
          player.z = msg.z;
          player.rotY = msg.rotY;
          player.lastUpdate = Date.now();
        } else {
          player = {
            pubkey: msg.pubkey,
            displayName: msg.pubkey,
            x: msg.x,
            y: msg.y,
            z: msg.z,
            rotY: msg.rotY,
            lastUpdate: Date.now(),
          };
          remotePlayers.set(msg.pubkey, player);
          for (const cb of joinCallbacks) cb(player);
        }
        for (const cb of moveCallbacks) cb(player);
        break;
      }

      case "player_list": {
        if (!Array.isArray(msg.players)) break;
        for (const p of msg.players) {
          const existing = remotePlayers.get(p.pubkey);
          if (existing) {
            existing.x = p.x;
            existing.y = p.y;
            existing.z = p.z;
            existing.rotY = p.rotY;
            existing.displayName = p.displayName;
            existing.lastUpdate = Date.now();
          } else {
            const player: RemotePlayer = {
              pubkey: p.pubkey,
              displayName: p.displayName,
              x: p.x ?? 0,
              y: p.y ?? 0,
              z: p.z ?? 0,
              rotY: p.rotY ?? 0,
              lastUpdate: Date.now(),
            };
            remotePlayers.set(p.pubkey, player);
            for (const cb of joinCallbacks) cb(player);
          }
        }
        break;
      }
    }
  } catch {
    // Ignore malformed messages
  }
}

function scheduleReconnect(): void {
  if (reconnectTimer !== null) return;

  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    openSocket();
  }, reconnectDelay);

  reconnectDelay = Math.min(reconnectDelay * 2, 30_000);
}

function startStaleCleanup(): void {
  if (staleCleanupTimer !== null) return;

  staleCleanupTimer = setInterval(() => {
    const now = Date.now();
    for (const [pubkey, player] of remotePlayers) {
      if (now - player.lastUpdate > 10_000) {
        remotePlayers.delete(pubkey);
        for (const cb of leaveCallbacks) cb(pubkey);
      }
    }
  }, 2000);
}

function stopStaleCleanup(): void {
  if (staleCleanupTimer !== null) {
    clearInterval(staleCleanupTimer);
    staleCleanupTimer = null;
  }
}
