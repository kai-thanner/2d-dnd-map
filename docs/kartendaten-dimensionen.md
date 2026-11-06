# Kartendaten & Dimensionen

Festgehaltener Maßstab und die daraus errechneten Größen. Stand: Branch `vibe/sloped-terrain-view-23ac3f`.

## 1. Feste Maße (Design-Entscheidung, nicht einstellbar)

| Größe | Wert | Bedeutung |
|---|---|---|
| 1 Kachel | **14 px × 14 px** | Spielfläche für den Client (Wald, Stadtteil, Dungeon über mehrere Kacheln …) |
| 1 Kachel in Metern | **1000 m × 1000 m = 1 km²** | `TILE_SIZE_M = 1000.0` in `src/config.rs` |
| Kartenbreite | `width` Kacheln | 100 Kacheln = **100 km** breit |
| Kartenfläche | `width × height` Kacheln | 100 × 100 = 100 km × 100 km = **10.000 km²** |
| max. Berghöhe | `max_elevation_m` = `H`, bis **6000 m über Wasserspiegel** | GUI-Slider 500..6000 m, CLI `--max-elevation` |
| Wasserspiegel | `sea_level` = `sea` (Anteil 0..1 der normierten Höhe) = **0 m** | Kachel ist Wasser, wenn `elevation < sea` |
| Höhenfaktor (3D) | **100 % = real 1:1**, fix `hf = 1.0` | 0 % wäre flache Fläche |

## 2. Gespeicherte Werte (pro Kachel, in `Tile` / Save-JSON)

`elevation` (0..1 normiert), `moisture` (0..1), `temperature` (°C), `biome`, `water` (Ocean/Lake/River/Ice), `river`, `river_width`, `road`, `settlement`, `poi`.

Die **Wassertiefe ist als Wert gespeichert** (in `elevation` unter `sea`), wird aber **nicht als Geometrie gerendert** — die Wasserfläche ist in der 3D-Ansicht eine Ebene auf Wasserspiegel-Höhe; die Tiefe steht nur als „Tiefe: X m" im Kachel-Info.

## 3. Errechnete Kartendaten & Formeln

Mit `e` = normierte `elevation` (0..1), `sea` = `sea_level`, `H` = `max_elevation_m`, `TILE_SIZE_M` = 1000 m:

**Höhenmodell (3D-Ansicht + Höhenanzeige, 0 m = Wasserspiegel):**

| Größe | Formel | Beispiel (sea=0,5, H=6000 m) |
|---|---|---|
| Höhe über Wasser einer Kachel | `(e − sea) / (1 − sea) × H` | e=1 → **6000 m** |
| Wassertiefe einer Kachel | `(sea − e) × H` | e=0 → **3000 m** |
| max. Berghöhe über Wasser | `H` (bei e = 1) | 6000 m |
| max. Wassertiefe („Rest") | `sea × H` | 0,5 × 6000 = **3000 m** |
| 3D-Gesamthöhe (Spanne) | `(1 + sea) × H` | 1,5 × 6000 = **9000 m** |
| Berghöhe in Kacheln (3D) | `H / TILE_SIZE_M` | 6000/1000 = **6 Kacheln** |
| Wassertiefe in Kacheln | `sea × H / TILE_SIZE_M` | 3 Kacheln (nur als Wert, nicht gerendert: `disp_e` clamp ≥ 0) |
| Höhe auf dem Bildschirm | `disp_e(e) × hf × cell` px | hf=1, cell=14 px → 6 Kacheln = 84 px |

`disp_e(e)` (in `src/view3d.rs`): Land `(e−sea)/(1−sea) × e_scale` mit `e_scale = H / TILE_SIZE_M`; Wasser wird auf 0 geclampt.

**Kartenmaße:**

| Größe | Formel | Beispiel 100×100 |
|---|---|---|
| Breite/Höhe in km | `width × TILE_SIZE_M / 1000` | 100 km |
| Fläche in km² | `width × height × TILE_SIZE_M² / 1e6` | 10.000 km² |

**Biome-Schwellen (lineare Skala, nur Generierung):** `h_m = (e − sea) × H`; Mountain ab `h = e − sea ≥ 0,34`; Rock ab `h_m > Baumgrenze + 800 m`; Baumgrenze effektiv `treeline_m × (0,8 + 0,5 × lat)` (Süden höher). Hinweis: Diese Biome-Skala ist linear über den ganzen Bereich; die 3D-Anzeige nutzt das piecewise-Modell oben.

**Terrain-Features:** 1 Noise-Einheit = `NOISE_TILES_PER_UNIT` = 25 Kacheln = 25 km — konstant in km, unabhängig von der Kartengröße.

## 4. Render-/Kamera-Formeln (`src/view3d.rs`)

- Projektion: erst **Yaw** (Drehung `rot_deg` um die senkrechte Achse) auf `(dx, dy)`, dann Kippen `ty = dy·sin(tilt) − e·hf·cos(tilt)` — seitlich kippt die Karte dadurch nicht auf den Kopf; bei 90° reine Draufsicht.
- Zoom an Mausposition (`zoom_at`) und Drehung um den Mauszeiger (`rotate_at`) halten den Ankerpunkt fix.

## 5. Noch offen (bewusst nicht umgesetzt)

- **Plattentektonik als zweite Heatmap** (Platten ~40 km, Berge an den Plattenrändern, innen flaches Land) — Modell besprochen, Code folgt nach Freigabe.
- **Berg-Footprint** („breit ≈ hoch ± 25 %") — aktuell bestimmt die Heightmap die Footprints von allein.
