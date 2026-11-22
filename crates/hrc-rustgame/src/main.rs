//! Horizon Rust (experimental): نموذج أولي للعبة مكعبات أصلية مكتوبة بـ Rust (wgpu + winit).
//! ليست Minecraft: لا تفتح عوالمه ولا تدخل سيرفراته ولا تشغّل مودات Java. كل الكود والعالم هنا من الصفر.
mod config;
mod font;
mod render;
mod world;

use anyhow::Context;
use config::{format_state, parse_state, Args, Config, SaveState};
use glam::{Mat4, Vec3};
use render::Renderer;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, WindowBuilder};
use world::World;

const VERSION: &str = "0.1";

struct Camera {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    fn forward(&self) -> Vec3 {
        Vec3::new(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin() * self.pitch.cos())
    }
    fn view_proj(&self, aspect: f32) -> [[f32; 4]; 4] {
        let view = Mat4::look_to_rh(self.pos, self.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(70f32.to_radians(), aspect, 0.1, 400.0);
        (proj * view).to_cols_array_2d()
    }
    fn state(&self) -> SaveState {
        SaveState { pos: self.pos.to_array(), yaw: self.yaw, pitch: self.pitch }
    }
}

#[derive(Default)]
struct Keys {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
    up: bool,
    down: bool,
    sprint: bool,
}

fn sanitize_user(u: &str) -> String {
    u.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').take(16).collect()
}

fn hud_lines(user: &str, fps: u32, cam: &Camera, chunks: usize, faces: usize, backend: &str) -> Vec<String> {
    vec![
        format!("HORIZON RUST {VERSION} (EXPERIMENTAL)"),
        format!("{fps} FPS"),
        format!("XYZ: {:.0} {:.0} {:.0}", cam.pos.x, cam.pos.y, cam.pos.z),
        format!("CHUNKS: {chunks}  FACES: {faces}"),
        format!("PLAYER: {user}  BACKEND: {}", backend.to_uppercase()),
    ]
}

fn request_adapter(instance: &wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> anyhow::Result<wgpu::Adapter> {
    for fallback in [false, true] {
        let opts = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface,
            force_fallback_adapter: fallback,
        };
        if let Some(a) = pollster::block_on(instance.request_adapter(&opts)) {
            return Ok(a);
        }
    }
    anyhow::bail!("no compatible GPU adapter (Vulkan / DX12 / Metal) found")
}

fn request_device(adapter: &wgpu::Adapter) -> anyhow::Result<(wgpu::Device, wgpu::Queue)> {
    Ok(pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor { label: None, required_features: wgpu::Features::empty(), required_limits: wgpu::Limits::default() },
        None,
    ))?)
}

fn spawn_camera(seed: u32) -> Camera {
    let h = world::height_at(8, 8, seed) as f32;
    Camera { pos: Vec3::new(8.0, h + 3.0, 8.0), yaw: 0.8, pitch: -0.2 }
}

/// بلا نافذة: يرسم إطاراً إلى texture ويحفظه PNG (للاختبار الآلي على CI عبر Vulkan البرمجي lavapipe).
fn headless(args: &Args, world: &World, mesh: &world::Mesh, path: &Path, user: &str) -> anyhow::Result<()> {
    let (w, h) = args.size.unwrap_or((1280, 720));
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = request_adapter(&instance, None)?;
    let info = adapter.get_info();
    println!("Adapter: {} ({:?}, {:?})", info.name, info.backend, info.device_type);
    let (device, queue) = request_device(&adapter)?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut r = Renderer::new(device, queue, format, (w, h), mesh);
    let tex = r.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    let mut cam = spawn_camera(world.seed);
    cam.pos += Vec3::new(0.0, 8.0, 0.0);
    cam.pitch = -0.35;
    let aspect = w as f32 / h as f32;
    let vp = cam.view_proj(aspect);

    // قياس زمن الإطار الفعلي (على lavapipe بطيء، وهذا متوقع)
    r.update(vp, &[]);
    let n = 8;
    let t0 = Instant::now();
    for _ in 0..n {
        r.render(&view);
        r.device().poll(wgpu::Maintain::Wait);
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let fps = (1000.0 / ms.max(0.001)) as u32;
    println!("Rendered {n} frames, {ms:.2} ms/frame ({fps} fps) at {w}x{h}");

    let lines = hud_lines(user, fps, &cam, world.chunk_count(), mesh.faces(), &format!("{:?}", info.backend));
    r.update(vp, &font::build_hud(&lines, (w, h)));
    r.render(&view);

    let padded = (w * 4).div_ceil(256) * 256;
    let buf = r.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * h) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut enc = r.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    enc.copy_texture_to_buffer(
        wgpu::ImageCopyTexture { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::ImageCopyBuffer { buffer: &buf, layout: wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(h) } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    r.queue().submit(Some(enc.finish()));
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    r.device().poll(wgpu::Maintain::Wait);
    rx.recv().context("readback channel")?.context("buffer map failed")?;
    let data = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for row in 0..h as usize {
        let start = row * padded as usize;
        pixels.extend_from_slice(&data[start..start + (w * 4) as usize]);
    }
    drop(data);
    buf.unmap();
    image::save_buffer(path, &pixels, w, h, image::ColorType::Rgba8).context("saving PNG")?;
    println!("Saved screenshot: {}", path.display());
    Ok(())
}

fn save_game(dir: &Option<PathBuf>, cam: &Camera) {
    if let Some(d) = dir {
        let _ = std::fs::create_dir_all(d);
        if let Err(e) = std::fs::write(d.join("horizon-rust-save.txt"), format_state(&cam.state())) {
            tracing::warn!("could not save game state: {e}");
        }
    }
}

fn run_window(args: &Args, cfg: &Config, world: &World, mesh: world::Mesh, user: String) -> anyhow::Result<()> {
    let event_loop = EventLoop::new().context("creating event loop")?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Horizon Rust (experimental)")
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .build(&event_loop)
            .context("creating window")?,
    );
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = instance.create_surface(window.clone()).context("creating surface")?;
    let adapter = request_adapter(&instance, Some(&surface))?;
    let info = adapter.get_info();
    println!("Adapter: {} ({:?}, {:?})", info.name, info.backend, info.device_type);
    let (device, queue) = request_device(&adapter)?;
    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
    let size = window.inner_size();
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: if cfg.vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync },
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);
    let faces = mesh.faces();
    let chunks = world.chunk_count();
    let mut renderer = Renderer::new(device, queue, format, (config.width, config.height), &mesh);

    let mut cam = spawn_camera(world.seed);
    if let Some(st) = args.dir.as_ref().and_then(|d| std::fs::read_to_string(d.join("horizon-rust-save.txt")).ok()).and_then(|t| parse_state(&t)) {
        cam = Camera { pos: Vec3::from_array(st.pos), yaw: st.yaw, pitch: st.pitch };
        println!("Loaded saved position from the instance folder");
    }
    let dir = args.dir.clone();
    let backend = format!("{:?}", info.backend);
    let mut keys = Keys::default();
    let mut grabbed = false;
    let (mut last, mut fps_start, mut fps_frames, mut fps) = (Instant::now(), Instant::now(), 0u32, 0u32);

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent { event: we, .. } => match we {
                    WindowEvent::CloseRequested => {
                        save_game(&dir, &cam);
                        elwt.exit();
                    }
                    WindowEvent::Resized(sz) if sz.width > 0 && sz.height > 0 => {
                        config.width = sz.width;
                        config.height = sz.height;
                        surface.configure(renderer.device(), &config);
                        renderer.resize(sz.width, sz.height);
                    }
                    WindowEvent::MouseInput { state: ElementState::Pressed, .. } if !grabbed => {
                        let ok = window.set_cursor_grab(CursorGrabMode::Locked).or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined)).is_ok();
                        if ok {
                            window.set_cursor_visible(false);
                            grabbed = true;
                        }
                    }
                    WindowEvent::KeyboardInput { event: ke, .. } => {
                        if let PhysicalKey::Code(code) = ke.physical_key {
                            let down = ke.state == ElementState::Pressed;
                            match code {
                                KeyCode::KeyW => keys.w = down,
                                KeyCode::KeyA => keys.a = down,
                                KeyCode::KeyS => keys.s = down,
                                KeyCode::KeyD => keys.d = down,
                                KeyCode::Space => keys.up = down,
                                KeyCode::ShiftLeft => keys.down = down,
                                KeyCode::ControlLeft => keys.sprint = down,
                                KeyCode::Escape if down => {
                                    if grabbed {
                                        let _ = window.set_cursor_grab(CursorGrabMode::None);
                                        window.set_cursor_visible(true);
                                        grabbed = false;
                                    } else {
                                        save_game(&dir, &cam);
                                        elwt.exit();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let dt = (now - last).as_secs_f32().min(0.1);
                        last = now;
                        let fwd = cam.forward();
                        let right = fwd.cross(Vec3::Y).normalize_or_zero();
                        let mut mv = Vec3::ZERO;
                        if keys.w { mv += fwd; }
                        if keys.s { mv -= fwd; }
                        if keys.d { mv += right; }
                        if keys.a { mv -= right; }
                        if keys.up { mv += Vec3::Y; }
                        if keys.down { mv -= Vec3::Y; }
                        cam.pos += mv.normalize_or_zero() * dt * if keys.sprint { 36.0 } else { 12.0 };

                        fps_frames += 1;
                        if fps_start.elapsed().as_secs_f32() >= 0.5 {
                            fps = (fps_frames as f32 / fps_start.elapsed().as_secs_f32()) as u32;
                            fps_frames = 0;
                            fps_start = Instant::now();
                        }
                        let aspect = config.width as f32 / config.height as f32;
                        let lines = hud_lines(&user, fps, &cam, chunks, faces, &backend);
                        renderer.update(cam.view_proj(aspect), &font::build_hud(&lines, (config.width, config.height)));
                        match surface.get_current_texture() {
                            Ok(frame) => {
                                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                                renderer.render(&view);
                                frame.present();
                            }
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => surface.configure(renderer.device(), &config),
                            Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            Err(e) => tracing::warn!("surface error: {e:?}"),
                        }
                    }
                    _ => {}
                },
                Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } if grabbed => {
                    cam.yaw += delta.0 as f32 * 0.0025;
                    cam.pitch = (cam.pitch - delta.1 as f32 * 0.0025).clamp(-1.55, 1.55);
                }
                Event::AboutToWait => window.request_redraw(),
                _ => {}
            }
        })
        .context("event loop")?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
    let args = Args::parse(std::env::args().skip(1)).map_err(|e| anyhow::anyhow!(e))?;

    let mut cfg = args
        .dir
        .as_ref()
        .and_then(|d| std::fs::read_to_string(d.join("horizon-rust.properties")).ok())
        .map(|t| Config::from_properties(&t))
        .unwrap_or_default();
    if let Some(s) = args.seed {
        cfg.seed = s;
    }
    if let Some(r) = args.radius {
        cfg.radius = r;
    }
    cfg.vsync |= args.vsync;
    let user = sanitize_user(args.user.as_deref().unwrap_or("Player"));

    let t0 = Instant::now();
    let mut world = World::new(cfg.seed);
    world.generate_around(0, 0, cfg.radius);
    let mesh = world.mesh_all();
    println!(
        "Horizon Rust (experimental) {VERSION}: seed {} radius {} -> {} chunks, {} faces in {} ms",
        cfg.seed,
        cfg.radius,
        world.chunk_count(),
        mesh.faces(),
        t0.elapsed().as_millis()
    );

    if let Some(path) = args.screenshot.clone() {
        return headless(&args, &world, &mesh, &path, &user);
    }
    run_window(&args, &cfg, &world, mesh, user)
}
