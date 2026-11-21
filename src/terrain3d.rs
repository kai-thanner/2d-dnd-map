//! Echtes 3D-Terrain via wgpu: ein einzelnes Heightfield-Mesh, ein Draw-Call,
//! Depth-Buffer + Lambert-Shading + Distanz-Nebel. Wird als egui-PaintCallback
//! in die bestehende egui-Renderpass gezeichnet (siehe `src/bin/egui.rs`).
//!
//! Die 2.5D-Ansicht (`view3d.rs`) bleibt unangetastet ("eingefroren").

use crate::palette;
use crate::world::World;
use eframe::egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use eframe::wgpu;
use eframe::wgpu::util::DeviceExt;
use std::sync::Arc;

/// Depth-Format der egui-Renderpass (muss zu `RendererOptions.depth_stencil_format` passen).
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

// ---------------------------------------------------------------------------
// Reine Mathematik (einheit-testbar, kein GPU noetig)
// ---------------------------------------------------------------------------

/// 4x4-Matrix, spalten-major: m[col][row].
pub type Mat4 = [[f32; 4]; 4];

pub fn mat4_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut m = [[0.0f32; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            m[c][r] = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    m
}

pub fn mat4_transform(m: &Mat4, v: [f32; 4]) -> [f32; 4] {
    let mut o = [0.0f32; 4];
    for r in 0..4 {
        o[r] = (0..4).map(|c| m[c][r] * v[c]).sum();
    }
    o
}

/// wgpu-Perspektive (Clip-Z in [0, 1]).
pub fn perspective(fov_y_deg: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y_deg.to_radians() * 0.5).tan();
    let nf = 1.0 / (near - far);
    let mut m = [[0.0f32; 4]; 4];
    m[0][0] = f / aspect;
    m[1][1] = f;
    m[2][2] = far * nf;
    m[2][3] = -1.0;
    m[3][2] = near * far * nf;
    m
}

fn norm3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
    [v[0] / l, v[1] / l, v[2] / l]
}
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Look-At-Viewmatrix (rechtshaendisch, Kamera schaut entlang -Z).
pub fn look_at(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> Mat4 {
    let f = norm3([center[0] - eye[0], center[1] - eye[1], center[2] - eye[2]]);
    let s = norm3(cross3(f, up));
    let u = cross3(s, f);
    // View-Matrix (zeilen-major): Zeilen [s | u | -f], Translation -dot(s,eye), -dot(u,eye), dot(f,eye)
    // -> spalten-major m[col][row]:
    let mut m = [[0.0f32; 4]; 4];
    m[0][0] = s[0];
    m[1][0] = u[0];
    m[2][0] = -f[0];
    m[3][0] = -dot3(s, eye);
    m[0][1] = s[1];
    m[1][1] = u[1];
    m[2][1] = -f[1];
    m[3][1] = -dot3(u, eye);
    m[0][2] = s[2];
    m[1][2] = u[2];
    m[2][2] = -f[2];
    m[3][2] = dot3(f, eye);
    m[3][3] = 1.0;
    m
}

/// Orbit-Kamera: `tilt_deg` = Erhebungswinkel ueber Horizont (90 = Draufsicht,
/// 0 = Horizont) — gleiche Semantik wie die 2.5D-Ansicht. `rot_deg` = Yaw.
/// Gibt (view_proj, eye) zurueck.
pub fn orbit_camera(
    rot_deg: f32,
    tilt_deg: f32,
    dist: f32,
    center: [f32; 3],
    aspect: f32,
    map_diag: f32,
) -> (Mat4, [f32; 3]) {
    let yaw = rot_deg.to_radians();
    let pitch = tilt_deg.to_radians().clamp(0.0, std::f32::consts::PI / 2.0);
    let (sp, cp) = (pitch.sin(), pitch.cos());
    let (sy, cy) = (yaw.sin(), yaw.cos());
    let eye = [
        center[0] + dist * cp * sy,
        center[1] + dist * cp * cy,
        center[2] + dist * sp,
    ];
    // up-Vektor wechselt, damit look_at nie entartet
    let up = if sp > 0.7 { [0.0, 1.0, 0.0] } else { [0.0, 0.0, 1.0] };
    let view = look_at(eye, center, up);
    let near = (dist * 0.02).max(0.5);
    let far = dist * 4.0 + map_diag * 2.0 + 100.0;
    let proj = perspective(50.0, aspect.max(0.01), near, far);
    (mat4_mul(&proj, &view), eye)
}

// ---------------------------------------------------------------------------
// Terrain-Mesh
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [u8; 4],
}

pub struct TerrainMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub version: u64,
}

impl TerrainMesh {
    pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Unorm8x4,
                },
            ],
        }
    }
}

/// Baut das Heightfield-Mesh: ein Vertex pro Gitterpunkt (x, y), Hoehe = disp
/// (wie 2.5D), Farbe = Biome-Farbe der Kachel, Normalen per Zentraldifferenz.
pub fn build_terrain_mesh(w: &World, e_off: f32, e_scale: f32, version: u64) -> TerrainMesh {
    let (gw, gh) = (w.width + 1, w.height + 1);
    let z_at = |x: usize, y: usize| -> f32 {
        let e = w.get(x.min(w.width - 1), y.min(w.height - 1)).unwrap().elevation;
        let d = e - e_off;
        let v = if d >= 0.0 {
            d / (1.0 - e_off).max(0.01) * e_scale
        } else {
            d * e_scale
        };
        v.clamp(-8.0, 64.0)
    };
    let mut z = vec![0.0f32; gw * gh];
    for y in 0..gh {
        for x in 0..gw {
            z[y * gw + x] = z_at(x, y);
        }
    }
    let mut vertices = Vec::with_capacity(gw * gh);
    for y in 0..gh {
        for x in 0..gw {
            let i = y * gw + x;
            // Normalen per Zentraldifferenz
            let xl = if x > 0 { z[i - 1] } else { z[i] };
            let xr = if x + 1 < gw { z[i + 1] } else { z[i] };
            let yt = if y > 0 { z[i - gw] } else { z[i] };
            let yb = if y + 1 < gh { z[i + gw] } else { z[i] };
            let n = norm3([xl - xr, yt - yb, 2.0]);
            let t = w.get(x.min(w.width - 1), y.min(w.height - 1)).unwrap();
            let (r, g, b) = palette::shade_rgb(w, x.min(w.width - 1), y.min(w.height - 1), palette::biome_rgb(t.effective_biome()));
            vertices.push(Vertex {
                pos: [x as f32 - w.width as f32 / 2.0, y as f32 - w.height as f32 / 2.0, z[i]],
                normal: n,
                color: [r, g, b, 255],
            });
        }
    }
    let mut indices = Vec::with_capacity(w.width * w.height * 6);
    for y in 0..w.height {
        for x in 0..w.width {
            let v00 = (y * gw + x) as u32;
            let v10 = (y * gw + x + 1) as u32;
            let v01 = ((y + 1) * gw + x) as u32;
            let v11 = ((y + 1) * gw + x + 1) as u32;
            indices.extend_from_slice(&[v00, v10, v11, v00, v11, v01]);
        }
    }
    TerrainMesh {
        vertices,
        indices,
        version,
    }
}

// ---------------------------------------------------------------------------
// wgpu-Callback
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: Mat4,
    cam_pos: [f32; 3],
    _pad0: f32,
    light_dir: [f32; 3],
    _pad1: f32,
    fog_color: [f32; 3],
    fog_density: f32,
}

pub const SHADER_WGSL: &str = r#"
struct U {
    view_proj: mat4x4<f32>,
    cam_pos: vec3<f32>,
    light_dir: vec3<f32>,
    fog_color: vec3<f32>,
    fog_density: f32,
};
@group(0) @binding(0) var<uniform> u: U;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
};
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn vs(v: VIn) -> VOut {
    var o: VOut;
    o.clip = u.view_proj * vec4<f32>(v.pos, 1.0);
    o.normal = v.normal;
    o.color = v.color;
    o.world = v.pos;
    return o;
}

@fragment
fn fs(v: VOut) -> @location(0) vec4<f32> {
    let n = normalize(v.normal);
    let diff = max(dot(n, normalize(u.light_dir)), 0.0);
    var col = v.color.rgb * (0.38 + 0.62 * diff);
    // leichte Hang-Schattierung: steile Hänge dunkler
    let slope = 1.0 - n.z;
    col = col * (1.0 - 0.25 * clamp(slope, 0.0, 1.0));
    let dist = length(v.world - u.cam_pos);
    let f = 1.0 - exp(-dist * u.fog_density);
    col = mix(col, u.fog_color, clamp(f, 0.0, 1.0));
    return vec4<f32>(col, 1.0);
}
"#;

struct Gpu {
    version: u64,
    pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: u32,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

/// egui-PaintCallback fürs Terrain. Wird pro Frame neu gebaut (billig); die
/// GPU-Ressourcen liegen in den `CallbackResources` und werden nur bei neuem
/// Mesh (Version) neu erzeugt.
pub struct Terrain3d {
    pub mesh: Arc<TerrainMesh>,
    pub view_proj: Mat4,
    pub cam_pos: [f32; 3],
    pub light_dir: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_density: f32,
    pub target_format: wgpu::TextureFormat,
    pub msaa_samples: u32,
}

impl CallbackTrait for Terrain3d {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let uniforms = Uniforms {
            view_proj: self.view_proj,
            cam_pos: self.cam_pos,
            _pad0: 0.0,
            light_dir: self.light_dir,
            _pad1: 0.0,
            fog_color: self.fog_color,
            fog_density: self.fog_density,
        };
        let need_rebuild = match resources.get::<Gpu>() {
            Some(g) => g.version != self.mesh.version,
            None => true,
        };
        if need_rebuild {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("terrain3d"),
                source: wgpu::ShaderSource::Wgsl(SHADER_WGSL.into()),
            });
            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("terrain3d"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("terrain3d"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("terrain3d"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(TerrainMesh::vertex_layout())],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.target_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: self.msaa_samples.max(1),
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            });
            let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain3d-verts"),
                contents: bytemuck::cast_slice(&self.mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain3d-idx"),
                contents: bytemuck::cast_slice(&self.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain3d-uniform"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("terrain3d"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                }],
            });
            resources.insert(Gpu {
                version: self.mesh.version,
                pipeline,
                vertex_buf,
                index_buf,
                index_count: self.mesh.indices.len() as u32,
                uniform_buf,
                bind_group,
            });
        } else {
            // nur Uniforms aktualisieren (Kamera bewegt sich)
            queue.write_buffer(
                &resources.get::<Gpu>().unwrap().uniform_buf,
                0,
                bytemuck::bytes_of(&uniforms),
            );
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: eframe::epaint::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let Some(gpu) = resources.get::<Gpu>() else {
            return;
        };
        render_pass.set_pipeline(&gpu.pipeline);
        render_pass.set_bind_group(0, &gpu.bind_group, &[]);
        render_pass.set_vertex_buffer(0, gpu.vertex_buf.slice(..));
        render_pass.set_index_buffer(gpu.index_buf.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..gpu.index_count, 0, 0..1);
    }
}
