/* ========================================================================
 * WEB AUDIO SYNTHESIZER (Sci-Fi Micro-Sounds)
 * ======================================================================== */
let audioCtx = null;
let soundEnabled = true;

function initAudio() {
  if (!audioCtx) {
    audioCtx = new (window.AudioContext || window.webkitAudioContext)();
  }
}

function toggleAudio() {
  soundEnabled = !soundEnabled;
  document.getElementById('audioToggle').textContent = soundEnabled ? '🔊 SFX: ON' : '🔇 SFX: OFF';
}

function playLaser(freq = 600, duration = 0.1) {
  if (!soundEnabled) return;
  initAudio();
  const osc = audioCtx.createOscillator();
  const gain = audioCtx.createGain();
  osc.type = 'sawtooth';
  osc.frequency.setValueAtTime(freq, audioCtx.currentTime);
  osc.frequency.exponentialRampToValueAtTime(120, audioCtx.currentTime + duration);
  gain.gain.setValueAtTime(0.15, audioCtx.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.01, audioCtx.currentTime + duration);
  osc.connect(gain);
  gain.connect(audioCtx.destination);
  osc.start();
  osc.stop(audioCtx.currentTime + duration);
}

function playAlarm() {
  if (!soundEnabled) return;
  initAudio();
  const osc = audioCtx.createOscillator();
  const gain = audioCtx.createGain();
  osc.type = 'square';
  osc.frequency.setValueAtTime(880, audioCtx.currentTime);
  osc.frequency.setValueAtTime(440, audioCtx.currentTime + 0.1);
  gain.gain.setValueAtTime(0.2, audioCtx.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.01, audioCtx.currentTime + 0.25);
  osc.connect(gain);
  gain.connect(audioCtx.destination);
  osc.start();
  osc.stop(audioCtx.currentTime + 0.25);
}

/* ========================================================================
 * NAVIGATION TAB SWITCHING
 * ======================================================================== */
function switchTab(tabId) {
  document.querySelectorAll('.nav-btn').forEach(btn => btn.classList.remove('active'));
  document.querySelectorAll('.view-container').forEach(v => v.classList.remove('active'));

  if (tabId === 'arena') {
    document.querySelectorAll('.nav-btn')[0].classList.add('active');
    document.getElementById('view-arena').classList.add('active');
  } else if (tabId === 'studio') {
    document.querySelectorAll('.nav-btn')[1].classList.add('active');
    document.getElementById('view-studio').classList.add('active');
    updateManifestPreview();
  } else if (tabId === 'engines') {
    document.querySelectorAll('.nav-btn')[2].classList.add('active');
    document.getElementById('view-engines').classList.add('active');
    switchEngine('godot');
  }
}

/* ========================================================================
 * ARENA SIMULATOR PHYSICS & RENDERING
 * ======================================================================== */
const canvas = document.getElementById('simCanvas');
const ctx = canvas.getContext('2d');

const state = {
  gravity: 9.81,
  lowGravityMod: false,
  zombieMod: false,
  strikes: 0,
  player: {
    x: 480,
    y: 450,
    vx: 0,
    vy: 0,
    radius: 18,
    isGrounded: true
  },
  keys: {},
  particles: [],
  zombies: [],
  floatingTexts: []
};

window.addEventListener('keydown', (e) => {
  state.keys[e.code] = true;
  if (e.code === 'Space' && state.player.isGrounded) {
    state.player.vy = state.lowGravityMod ? -14 : -11;
    state.player.isGrounded = false;
    playLaser(500, 0.12);
    spawnParticles(state.player.x, state.player.y + 18, 12, '#00f0ff');
  }
});

window.addEventListener('keyup', (e) => {
  state.keys[e.code] = false;
});

function spawnParticles(x, y, count, color) {
  for (let i = 0; i < count; i++) {
    state.particles.push({
      x, y,
      vx: (Math.random() - 0.5) * 8,
      vy: (Math.random() - 0.5) * 8 - 2,
      radius: Math.random() * 3 + 1,
      color: color,
      life: 1.0,
      decay: Math.random() * 0.04 + 0.02
    });
  }
}

function addFloatingText(x, y, text, color) {
  state.floatingTexts.push({ x, y, text, color, life: 1.0 });
}

function addAuditLog(msg, type = 'allow') {
  const log = document.getElementById('auditLog');
  const now = new Date();
  const timeStr = now.toTimeString().split(' ')[0];
  const div = document.createElement('div');
  div.className = 'log-line';
  div.innerHTML = `
    <span class="log-time">[${timeStr}]</span>
    <span class="log-${type}">${msg}</span>
  `;
  log.appendChild(div);
  log.scrollTop = log.scrollHeight;
}

/* MOD INTERACTION LOGIC */
function toggleLowGravity() {
  state.lowGravityMod = !state.lowGravityMod;
  const btn = document.getElementById('btn-toggle-grav');
  const badge = document.getElementById('badge-low-grav');
  const gravVal = document.getElementById('gravityVal');
  const gravModState = document.getElementById('gravModState');

  if (state.lowGravityMod) {
    state.gravity = 2.40;
    btn.textContent = 'Unload Low-Gravity Mod';
    badge.textContent = 'INSTALLED (v1.3.0)';
    badge.className = 'mod-status installed';
    gravVal.textContent = '2.40 m/s²';
    gravVal.className = 'tele-value text-cyan';
    gravModState.textContent = 'LOW-GRAV MOD';
    addAuditLog("CALL world.set_gravity({ gravity: 2.40 }) -> GRANTED", "allow");
    addFloatingText(state.player.x, state.player.y - 40, "LOW GRAVITY ACTIVE (-75% G)", "#00f0ff");
    playLaser(800, 0.2);
  } else {
    state.gravity = 9.81;
    btn.textContent = 'Load Low-Gravity Mod';
    badge.textContent = 'UNLOADED';
    badge.className = 'mod-status uninstalled';
    gravVal.textContent = '9.81 m/s²';
    gravVal.className = 'tele-value text-amber';
    gravModState.textContent = 'VANILLA';
    addAuditLog("Module 'low-gravity' unloaded. Physics restored to 9.81 m/s²", "warn");
  }
}

function toggleZombieSpawner() {
  state.zombieMod = true;
  const badge = document.getElementById('badge-zombies');
  badge.textContent = 'ACTIVE';
  badge.className = 'mod-status installed';

  // Spawn 3 zombies
  for (let i = 0; i < 3; i++) {
    const x = i === 0 ? 100 : (i === 1 ? 860 : 200);
    state.zombies.push({
      id: Math.floor(Math.random() * 9000 + 1000),
      x: x,
      y: 455,
      vx: 0,
      speed: Math.random() * 1.5 + 1.2
    });
    spawnParticles(x, 455, 15, '#ff007f');
  }

  addAuditLog("EVENT round_start -> zombie-spawner emitted 3x world.spawn -> GRANTED", "allow");
  addFloatingText(480, 200, "WAVE 1 SPAWNED (3x CYBER-ZOMBIES)", "#ff007f");
  playLaser(350, 0.25);
}

function triggerHostileExploit() {
  playAlarm();
  const badge = document.getElementById('badge-malicious');
  badge.textContent = 'TRAPPED';
  badge.className = 'mod-status trapped';

  state.strikes = Math.min(state.strikes + 1, 3);
  document.getElementById('strikesVal').textContent = `${state.strikes} / 3 Strikes`;
  document.getElementById('strikesFill').style.width = `${(state.strikes / 3) * 100}%`;

  if (state.strikes >= 3) {
    document.getElementById('breakerState').textContent = 'TRIPPED';
    document.getElementById('breakerState').className = 'text-red';
  }

  addAuditLog("SECURITY ALERT: malicious-mod invoked net.http (SSRF) -> DENIED", "deny");
  addAuditLog("SECURITY TRAP: OutOfFuel trapped at 50,000 instructions! Frame preserved.", "deny");

  // Show alert overlay
  const alert = document.getElementById('shieldAlert');
  alert.style.display = 'block';
  spawnParticles(state.player.x, state.player.y, 40, '#ff3344');

  setTimeout(() => {
    alert.style.display = 'none';
  }, 2500);
}

/* MAIN GAME TICK LOOP (60 Ticks/sec) */
function gameLoop() {
  // 1. Player Movement
  if (state.keys['KeyA'] || state.keys['ArrowLeft']) {
    state.player.vx = -5;
  } else if (state.keys['KeyD'] || state.keys['ArrowRight']) {
    state.player.vx = 5;
  } else {
    state.player.vx *= 0.8;
  }

  state.player.x += state.player.vx;
  if (state.player.x < 30) state.player.x = 30;
  if (state.player.x > 930) state.player.x = 930;

  // 2. Gravity & Jump
  state.player.vy += state.gravity * 0.05;
  state.player.y += state.player.vy;

  const floorY = 460;
  if (state.player.y >= floorY) {
    state.player.y = floorY;
    state.player.vy = 0;
    state.player.isGrounded = true;
  }

  // 3. Update Zombies
  state.zombies.forEach(z => {
    if (z.x < state.player.x) z.x += z.speed;
    if (z.x > state.player.x) z.x -= z.speed;
  });

  // 4. Update Particles
  for (let i = state.particles.length - 1; i >= 0; i--) {
    const p = state.particles[i];
    p.x += p.vx;
    p.y += p.vy;
    p.life -= p.decay;
    if (p.life <= 0) state.particles.splice(i, 1);
  }

  // 5. Update Floating Texts
  for (let i = state.floatingTexts.length - 1; i >= 0; i--) {
    const ft = state.floatingTexts[i];
    ft.y -= 1.0;
    ft.life -= 0.015;
    if (ft.life <= 0) state.floatingTexts.splice(i, 1);
  }

  // 6. Render Canvas
  render();

  requestAnimationFrame(gameLoop);
}

function render() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  // Draw Cyberpunk Grid
  ctx.strokeStyle = 'rgba(0, 240, 255, 0.08)';
  ctx.lineWidth = 1;
  for (let x = 0; x < canvas.width; x += 40) {
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, canvas.height);
    ctx.stroke();
  }
  for (let y = 0; y < canvas.height; y += 40) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(canvas.width, y);
    ctx.stroke();
  }

  // Draw Floor Platform
  ctx.fillStyle = '#0a101d';
  ctx.fillRect(0, 480, canvas.width, 60);

  ctx.strokeStyle = state.lowGravityMod ? 'var(--neon-cyan)' : 'rgba(0, 240, 255, 0.4)';
  ctx.lineWidth = 2;
  ctx.shadowColor = state.lowGravityMod ? '#00f0ff' : 'transparent';
  ctx.shadowBlur = state.lowGravityMod ? 15 : 0;
  ctx.beginPath();
  ctx.moveTo(0, 480);
  ctx.lineTo(canvas.width, 480);
  ctx.stroke();
  ctx.shadowBlur = 0;

  // Draw Player
  ctx.save();
  ctx.shadowColor = '#00f0ff';
  ctx.shadowBlur = 20;
  ctx.fillStyle = '#00f0ff';
  ctx.beginPath();
  ctx.arc(state.player.x, state.player.y, state.player.radius, 0, Math.PI * 2);
  ctx.fill();

  // Inner Core
  ctx.fillStyle = '#ffffff';
  ctx.beginPath();
  ctx.arc(state.player.x, state.player.y, state.player.radius * 0.5, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();

  // Draw Zombies
  state.zombies.forEach(z => {
    ctx.save();
    ctx.shadowColor = '#ff007f';
    ctx.shadowBlur = 15;
    ctx.fillStyle = '#ff007f';
    ctx.beginPath();
    ctx.rect(z.x - 12, z.y - 12, 24, 24);
    ctx.fill();

    // Eye
    ctx.fillStyle = '#ffe600';
    ctx.beginPath();
    ctx.arc(z.x, z.y, 4, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
  });

  // Draw Particles
  state.particles.forEach(p => {
    ctx.save();
    ctx.globalAlpha = p.life;
    ctx.fillStyle = p.color;
    ctx.beginPath();
    ctx.arc(p.x, p.y, p.radius, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
  });

  // Draw Floating Texts
  state.floatingTexts.forEach(ft => {
    ctx.save();
    ctx.globalAlpha = ft.life;
    ctx.fillStyle = ft.color;
    ctx.font = 'bold 16px Outfit, sans-serif';
    ctx.textAlign = 'center';
    ctx.shadowColor = ft.color;
    ctx.shadowBlur = 10;
    ctx.fillText(ft.text, ft.x, ft.y);
    ctx.restore();
  });
}

requestAnimationFrame(gameLoop);

/* ========================================================================
 * CAPABILITY STUDIO YAML GENERATOR
 * ======================================================================== */
function updateManifestPreview() {
  const g = document.getElementById('perm-gravity').checked;
  const s = document.getElementById('perm-spawn').checked;
  const u = document.getElementById('perm-ui').checked;
  const n = document.getElementById('perm-net').checked;
  const f = document.getElementById('perm-fs').checked;

  let caps = [];
  if (g) caps.push(`  - name: world.set_gravity\n    category: gameplay\n    max_calls_per_tick: 5\n    location: ServerAndClient`);
  if (s) caps.push(`  - name: world.spawn\n    category: entities\n    max_calls_per_tick: 10\n    location: ServerOnly`);
  if (u) caps.push(`  - name: ui.notify\n    category: ui\n    max_calls_per_tick: 2\n    location: ClientOnly`);
  if (n) caps.push(`  - name: net.http\n    category: network\n    max_calls_per_tick: 0 # DENIED BY DEFAULT`);
  if (f) caps.push(`  - name: fs.raw_disk\n    category: storage\n    max_calls_per_tick: 0 # DENIED BY DEFAULT`);

  const yaml = `schema_version: "1.0.0"
contract_id: "custom-game-v1"
runtime:
  wasm_engine: "wasmtime-48"
  fuel_limit_per_tick: 100000
  memory_limit_pages: 256 # 16 MB max linear memory
  circuit_breaker_threshold: 3

capabilities:
${caps.join('\n\n')}`;

  document.getElementById('yamlPreview').textContent = yaml;
}

/* ========================================================================
 * ENGINE BRIDGES PREVIEW
 * ======================================================================== */
const codeSnippets = {
  godot: `extends Node

@onready var worldvm = WorldVM.new()

func _ready():
    # 1. Initialize runtime
    worldvm.initialize()

    # 2. Expose safe engine capability
    worldvm.expose("world.set_gravity", func(input):
        var gravity = input.get("gravity", 9.81)
        PhysicsServer3D.area_set_param(
            get_viewport().find_world_3d().space, 
            PhysicsServer3D.AREA_PARAM_GRAVITY, 
            gravity
        )
    )

    # 3. Load creator package
    worldvm.load_package("res://mods/low-gravity.worldmod")

    # 4. Dispatch gameplay event
    worldvm.emit_event("round_start", { "round": 1 })`,

  unity: `using UnityEngine;
using WorldVM;

public class GameController : MonoBehaviour
{
    void Start()
    {
        // 1. Initialize runtime with contract
        WorldVMRuntime.Initialize();

        // 2. Load creator .worldmod package
        byte[] packageBytes = System.IO.File.ReadAllBytes("Assets/Mods/low-gravity.worldmod");
        WorldVMRuntime.LoadModule(packageBytes);

        // 3. Emit match event
        WorldVMRuntime.EmitEvent("low-gravity", "round_start", "{}");
    }
}`,

  unreal: `#include "WorldVMSubsystem.h"

void AMyGameMode::BeginPlay()
{
    Super::BeginPlay();

    UWorldVMSubsystem* WorldVM = GetGameInstance()->GetSubsystem<UWorldVMSubsystem>();
    if (WorldVM)
    {
        // 1. Initialize runtime
        WorldVM->InitializeRuntime();

        // 2. Load creator .worldmod
        WorldVM->LoadWorldMod(TEXT("Content/Mods/low-gravity.worldmod"));

        // 3. Emit event
        WorldVM->EmitWorldVMEvent(TEXT("low-gravity"), TEXT("round_start"), TEXT("{}"));
    }
}`,

  rust: `use worldvm_runtime::WorldVmRuntime;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_package::WorldModPackage;

// 1. Define game contract
let contract = WorldCapabilityContract::standard_arcade_contract("neon-arena");
let mut runtime = WorldVmRuntime::new(contract, my_game_host, false)?;

// 2. Load creator .worldmod
let pkg = WorldModPackage::from_file("mods/low-gravity.worldmod")?;
runtime.load_module(pkg)?;

// 3. Emit 60Hz tick event
runtime.emit_event("low-gravity", "round_start", b"{}")?;`
};

function switchEngine(engine) {
  document.querySelectorAll('.engine-tab').forEach(tab => tab.classList.remove('active'));
  const tabs = document.querySelectorAll('.engine-tab');
  if (engine === 'godot') tabs[0].classList.add('active');
  if (engine === 'unity') tabs[1].classList.add('active');
  if (engine === 'unreal') tabs[2].classList.add('active');
  if (engine === 'rust') tabs[3].classList.add('active');

  document.getElementById('engineCodePreview').textContent = codeSnippets[engine];
}

// Initial setup
updateManifestPreview();
switchEngine('godot');
