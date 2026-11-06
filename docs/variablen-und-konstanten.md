# Variablen & Konstanten – Übersicht

Stand: Branch `vibe/sloped-terrain-view-23ac3f`. Alle Angaben beziehen sich auf den Code in `src/`.

## 1. Feste Konstanten

| Konstante | Wert | Datei | Bedeutung |
|---|---|---|---|
| `TILE_SIZE_M` | `1000.0` m | `src/config.rs` | **Fester Maßstab: 14 px = 1 Kachel = 1000 m Kantenlänge** (1 Kachel = 1 km² Spielfläche). Nicht einstellbar. |
| `NOISE_TILES_PER_UNIT` | `25.0` Kacheln | `src/gen/heightmap.rs` | 1 Noise-Einheit = 25 Kacheln = 25 km. Terrain-Features sind damit in km konstant, **unabhängig von der Kartengröße**. |
| `MAX_RIVER_WIDTH` | `8` | `src/gen/rivers.rs` | Max. Flussbreite in Kacheln (Overflow-Schutz). |
| `SUB` | `3` | `src/bin/egui.rs` | Sub-Pixel pro Kachel bei Zoom ≥ 8 px (scharfe Küsten/Klippen). |
| `SAVE_VERSION` | `1` | `src/save.rs` | Version des Speicherformats (JSON). |
| `hf` (Höhenfaktor, in `View3D`) | `1.0` | `src/view3d.rs` | Fest auf **100 % = real 1:1**. Kein Slider mehr. |

## 2. Config (Generierung) – `src/config.rs`, CLI-Defaults in Klammern

| Variable | Typ | Default (CLI) | Bedeutung |
|---|---|---|---|
| `seed` | u64 | 1 | Zufalls-Seed für die gesamte Generierung |
| `width` / `height` | usize | 120 / 60 | Kartengröße in **Kacheln** (100 breit = 100 km breit) |
| `sea_level` | f32 0..1 | 0.42 | **Wasserspiegel als Anteil** der normalisierten Höhe (0,5 = 50 %). Kachel ist Wasser, wenn `elevation < sea_level` |
| `moisture_scale` | f32 0..1 | 0.6 | Skalierung der Feuchtigkeits-Noise |
| `dry_south` | f32 0..1 | 0.45 | Trockenheits-Gradient Richtung Süden (Wüsten) |
| `river_amount` | f32 0..2 | 0.5 | Flussmenge |
| `max_elevation_m` | f32 m | 3000 (GUI bis 6000) | **Max. Berghöhe über dem Wasserspiegel in Metern** (e = 1) |
| `treeline_m` | f32 m | 2000 | Baumgrenze in Metern (breitengrad-abhängig, s. Kartendaten) |
| `settlement_density` | f32 0..1 | 0.5 | Besiedelungsdichte |
| `climate.south_temp` | °C | 28 | Temperatur am Südrand |
| `climate.north_temp` | °C | −6 | Temperatur am Nordrand |
| `climate.lapse_per_unit` | °C | 45 | Temperaturabfall pro Höhen-Einheit |
| `climate.snow_temp` | °C | 0 | Schneegrenze |

## 3. World / Tile (pro Kachel gespeichert) – `src/world.rs`

| Feld | Typ | Bedeutung |
|---|---|---|
| `World.width/height` | usize | Kacheln |
| `World.sea_level` | f32 | Wasserspiegel (Anteil 0..1) |
| `Tile.elevation` | f32 0..1 | **normalisierte Höhe** (0 = tiefstes Wasser, 1 = Gipfel); Meterwert: `(e − sea_level) × max_elevation_m` |
| `Tile.moisture` | f32 0..1 | Feuchtigkeit |
| `Tile.temperature` | f32 | °C |
| `Tile.biome` | Biome | Land-Biom (bleibt unter Wasser-Layer erhalten) |
| `Tile.water` | Option<WaterKind> | Wasser-Overlay: Ocean / Lake / River / Ice |
| `Tile.river` / `river_width` | bool / u32 | Fluss-Flag / Breite (≤ 8) |
| `Tile.road` | bool | Straße |
| `Tile.settlement` | Option<Settlement> | `name`, `population`, `has_port` |
| `Tile.poi` | Option<Poi> | Dungeon{name} / Inn{name} / Merchant{name, goods} |

`Tile.effective_biome()` = Wasser-Layer überschreibt Land-Biom. `is_passable()` = nicht Ocean/Lake/Ice/Mountain.

## 4. View3D (Kamera/Ansicht) – `src/view3d.rs`

| Feld | Default | Bedeutung |
|---|---|---|
| `cell` | 14 px | px pro Kachel (Zoom-Slider 4..40) |
| `pan` | (0,0) | Verschiebung in px (mittlere Maustaste) |
| `rot_deg` | 0 | Drehung als **Yaw um die senkrechte Achse** (vor dem Kippen angewendet), 0..360° |
| `tilt_deg` | 60 | Kippwinkel 0..90° (90° = Draufsicht/Spielfläche) |
| `hf` | 1.0 | Höhenfaktor, fix 100 % real |
| `e_off` | = `sea_level` | Höhen-Offset: Wasserspiegel = 0 m |
| `e_scale` | = `max_elevation_m / TILE_SIZE_M` | Kacheln pro voller Berghöhe über Wasser (6000 m → 6 Kacheln) |

`disp_e(e)` = Höhe über Wasser in Kacheln, **geclampt auf ≥ 0** (Wassertiefe wird nicht gerendert).

## 5. Eingabe-Belegung (egui)

- Mausrad = Zoom an Mausposition
- mittlere Maustaste = Karte verschieben (Pan)
- **Shift + mittlere Maustaste** ziehen = hoch/runter Kippen 0–90°, links/rechts Drehung um den Mauszeiger
- Q/E = Drehung um Mauszeiger; W/S oder Pfeile hoch/runter = Kippen

## 6. Straßen-Kosten (`step_cost`, `src/gen/roads.rs`)

| Gelände | Kosten |
|---|---|
| Ocean / Ice | **∞ (nie)** – keine Straßen übers Meer |
| Lake | 6, falls schiffbares Binnengewässer (Hafen-Flut), sonst ∞ |
| River | 15 (Brücke) |
| Mountain | 100 |
| Rock | 25 |
| Forest / Jungle / Taiga | 3 |
| Desert | 2 |
| sonst (Wiese etc.) | 1 |
| + Höhen-Delta | `|Δe| × 6` |
| + Jitter (Noise) | `fbm × 6` |
| auf bestehender Straße/Siedlung | × 0,5 (Rabatt) |

## 7. Generierung (Daumenwerte)

- Dungeons: `max(2, round(settlement_density × 6))`, nur in Mountain/Rock/Forest/Taiga, ≥ 8 Kacheln von Siedlungen
- Gasthäuser: `min(8, Siedlungen + 2)`, nur an Straßen, 2–14 Kacheln von Siedlung
- Händler: einer pro Siedlung; **15 % Chance auf berühmten Händler** (`FAMOUS_MERCHANTS` in `names.rs`)
- Hafen: jede besiedelte Landmasse bekommt mind. 1 `has_port`
- Siedlungs-Stufen nach Einwohnerzahl (Gehöft/Dorf/Kleinstadt/Stadt/Kaiserstadt, + Hafenstadt an Küste)
