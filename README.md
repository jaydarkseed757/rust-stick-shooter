# Vector Storm

A Robotron-style twin-stick arcade shooter written in Rust. Pure vector graphics — no external images or audio files. Everything is drawn with primitives and all sound effects are synthesized in-engine using a custom chiptune generator.

## Features

- Single-screen arena with progressive wave difficulty
- Dual shoot input: mouse aim + click, arrow keys, or right gamepad stick simultaneously
- Gamepad support (left stick move, right stick aim/fire)
- 6 enemy types with distinct AI and attack patterns
- Particle death explosions
- 10-slot high score leaderboard with initials entry
- Attract mode: title screen cycles to high scores after 30 seconds
- Procedural chiptune sound effects (square wave + LFSR noise, no audio files)
- 3D wireframe ship on title screen with parallax star field

## Prerequisites

**Rust toolchain** — [rustup.rs](https://rustup.rs)

**Linux system libraries:**
```bash
sudo apt-get install -y libudev-dev libasound2-dev
```
> `libudev-dev` is required by gilrs (gamepad input).  
> `libasound2-dev` is required by rodio (audio output via ALSA).

macOS and Windows do not need these — their audio and HID backends are built in.

## Build & Run

```bash
git clone https://github.com/jaydarkseed757/rust-stick-shooter.git
cd rust-stick-shooter
git checkout claude/twin-stick-shooter-rust-U1M7W
cargo run --release
```

Use `--release` for the smooth frame rate the game is designed around. Debug builds will feel sluggish.

## Controls

| Action | Keyboard / Mouse | Gamepad |
|---|---|---|
| Move | WASD | Left stick |
| Aim & shoot | Mouse (hold left click or Space) | Right stick (auto-fires) |
| Shoot (digital) | Arrow keys | — |
| Start / confirm | Enter or Space | Start / South button |
| Credits | C (title screen) | — |

Both mouse and arrow-key shooting are active at the same time — hold arrows to spray while aiming with the mouse.

## Enemies

| Enemy | Colour | Behaviour | First Wave |
|---|---|---|---|
| Grunt | Red triangle | Charges directly at player | 1 |
| Spheroid | Purple circle | Arcs around arena, spawns Grunts | 2 |
| Tank | Green hexagon | Slow, fires single aimed shot | 3 |
| Enforcer | Orange diamond | Fast diagonal bouncer, 3-way spread shot | 4 |
| Phantom | Magenta rings | Teleports periodically, fires 4-cardinal burst | 5 |
| Bomber | Gold circle | Very slow, fires 8-way radial burst | 6 |

## High Scores

Scores are saved to `scores.dat` in the working directory (plain text, one entry per line). The leaderboard holds 10 entries. If no file exists, a set of default scores is used so the board is never empty.

When the game ends, you go straight to the initials entry screen if your score qualifies, then to the full leaderboard.

## Project Structure

```
src/
  main.rs     — game loop, all game logic and rendering
  sound.rs    — chiptune engine (ChipSource, Mix2, SoundSystem)
scores.dat    — high score file (created at runtime)
```

## Credits

Design & code — **Jay**  
Additional help — **Claude** (Anthropic)  
Built with [macroquad](https://macroquad.rs), [gilrs](https://gitlab.com/gilrs-project/gilrs), [rodio](https://github.com/RustAudio/rodio)
