// Proto — Crossworld clean-room web client
// Entry point: login, init WASM, scene, game loop

import * as THREE from 'three';
import { createScene, render as renderScene } from './scene';
import {
  loginWithExtension,
  loginAsGuest,
  joinLiveEvent,
  type NostrUser,
} from './nostr';
import {
  initPlayer,
  setupControls,
  updatePlayer,
  getPlayerRotation,
} from './player';
import { initWorld, createWorldMesh } from './world';
import { initAvatars, createAvatarInstance, getDefaultAvatar } from './avatar';
import {
  connect,
  sendPosition,
  onPlayerJoin,
  onPlayerLeave,
  onPlayerMove,
  getPlayerCount,
  type RemotePlayer,
} from './network';

// --- DOM refs ---

const loginScreen = document.getElementById('login-screen')!;
const btnExtension = document.getElementById('btn-extension')!;
const btnGuest = document.getElementById('btn-guest')!;
const crosshair = document.getElementById('crosshair')!;
const hud = document.getElementById('hud')!;
const hudPos = document.getElementById('hud-pos')!;
const hudFps = document.getElementById('hud-fps')!;
const hudPlayers = document.getElementById('hud-players')!;

// --- State ---

let running = false;
let lastTime = 0;
let sceneRef: THREE.Scene | null = null;
let cameraRef: THREE.PerspectiveCamera | null = null;

const remoteAvatars = new Map<string, THREE.Group>();
let defaultAvatarTemplate: THREE.Group | null = null;

// FPS tracking
let frameCount = 0;
let fpsAccum = 0;
let displayFps = 0;

// --- Login ---

btnExtension.addEventListener('click', async () => {
  try {
    const user = await loginWithExtension();
    await startGame(user);
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : 'Login failed';
    alert(msg);
  }
});

btnGuest.addEventListener('click', async () => {
  const user = loginAsGuest();
  await startGame(user);
});

// --- Game init ---

async function startGame(user: NostrUser): Promise<void> {
  loginScreen.style.display = 'none';
  crosshair.style.display = '';
  hud.style.display = '';

  // Create Three.js scene
  const { scene, camera, canvas } = createScene();
  sceneRef = scene;
  cameraRef = camera;

  // Init WASM modules in parallel
  await Promise.all([initWorld(), initPlayer(), initAvatars()]);

  // Build world terrain mesh
  createWorldMesh(scene);

  // Player controls (pointer lock, WASD, gamepad)
  setupControls(canvas, camera);

  // Pre-load default avatar for remote players
  try {
    defaultAvatarTemplate = await getDefaultAvatar();
  } catch {
    // Avatars not available — remote players render without mesh
  }

  // Network callbacks
  onPlayerJoin((p) => addRemoteAvatar(p));
  onPlayerLeave((pubkey) => removeRemoteAvatar(pubkey));
  onPlayerMove((p) => moveRemoteAvatar(p));

  // Connect to game server (best-effort — works offline too)
  try {
    connect('ws://localhost:4433', user.pubkey, user.displayName);
  } catch {
    // Offline mode
  }

  // Join Nostr live event for chat
  joinLiveEvent().catch(() => {});

  // Start loop
  running = true;
  lastTime = performance.now();
  requestAnimationFrame(gameLoop);
}

// --- Game loop ---

function gameLoop(now: number): void {
  if (!running) return;
  requestAnimationFrame(gameLoop);

  const dt = Math.min((now - lastTime) / 1000, 0.1);
  lastTime = now;

  // Physics + camera update
  const pos = updatePlayer(dt, cameraRef!);

  // Broadcast position to server
  sendPosition(pos.x, pos.y, pos.z, getPlayerRotation());

  // FPS counter
  frameCount++;
  fpsAccum += dt;
  if (fpsAccum >= 1) {
    displayFps = Math.round(frameCount / fpsAccum);
    frameCount = 0;
    fpsAccum = 0;
  }

  // HUD
  hudPos.textContent = `Pos: ${pos.x.toFixed(1)}, ${pos.y.toFixed(1)}, ${pos.z.toFixed(1)}`;
  hudFps.textContent = `FPS: ${displayFps}`;
  hudPlayers.textContent = `Players: ${getPlayerCount()}`;

  // Render
  renderScene();
}

// --- Remote avatars ---

function addRemoteAvatar(player: RemotePlayer): void {
  if (remoteAvatars.has(player.pubkey) || !defaultAvatarTemplate || !sceneRef) return;

  const avatar = createAvatarInstance(player.pubkey, defaultAvatarTemplate);
  avatar.position.set(player.x, player.y, player.z);
  sceneRef.add(avatar);
  remoteAvatars.set(player.pubkey, avatar);
}

function removeRemoteAvatar(pubkey: string): void {
  const avatar = remoteAvatars.get(pubkey);
  if (!avatar || !sceneRef) return;
  sceneRef.remove(avatar);
  remoteAvatars.delete(pubkey);
}

function moveRemoteAvatar(player: RemotePlayer): void {
  let avatar = remoteAvatars.get(player.pubkey);
  if (!avatar) {
    addRemoteAvatar(player);
    avatar = remoteAvatars.get(player.pubkey);
    if (!avatar) return;
  }
  avatar.position.set(player.x, player.y, player.z);
  avatar.rotation.y = player.rotY;
}
