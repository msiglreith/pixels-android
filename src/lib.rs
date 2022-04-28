#![deny(clippy::all)]

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, TouchPhase, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;
const BOX_SIZE: i16 = 64;

/// Representation of the application state. In this example, a box will bounce around the screen.
struct World {
    box_x: i16,
    box_y: i16,
    velocity_x: i16,
    velocity_y: i16,
}

#[cfg_attr(
    target_os = "android",
    ndk_glue::main(backtrace = "on", logger(tag = "pixels-android", level = "info"))
)]
fn main() {
    run().unwrap();
}

fn show_soft_input(show: bool) -> bool {
    let ctx = ndk_glue::native_activity();

    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }.unwrap();
    let env = vm.attach_current_thread().unwrap();

    let class_ctxt = env.find_class("android/content/Context").unwrap();
    let ime = env
        .get_static_field(class_ctxt, "INPUT_METHOD_SERVICE", "Ljava/lang/String;")
        .unwrap();
    let ime_manager = env
        .call_method(
            ctx.activity(),
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[ime],
        )
        .unwrap()
        .l()
        .unwrap();

    let jni_window = env
        .call_method(ctx.activity(), "getWindow", "()Landroid/view/Window;", &[])
        .unwrap()
        .l()
        .unwrap();
    let view = env
        .call_method(jni_window, "getDecorView", "()Landroid/view/View;", &[])
        .unwrap()
        .l()
        .unwrap();

    if show {
        let result = env
            .call_method(
                ime_manager,
                "showSoftInput",
                "(Landroid/view/View;I)Z",
                &[view.into(), 0i32.into()],
            )
            .unwrap()
            .z()
            .unwrap();
        log::info!("show input: {}", result);
        result
    } else {
        let window_token = env
            .call_method(view, "getWindowToken", "()Landroid/os/IBinder;", &[])
            .unwrap()
            .l()
            .unwrap();
        let result = env
            .call_method(
                ime_manager,
                "hideSoftInputFromWindow",
                "(Landroid/os/IBinder;I)Z",
                &[window_token.into(), 0i32.into()],
            )
            .unwrap()
            .z()
            .unwrap();
        log::info!("hide input: {}", result);
        result
    }
}

fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Hello Pixels")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels: Option<Pixels> = None;
    let mut world = World::new();

    let mut soft_keyboard = false;

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        if let Event::Resumed = event {
            log::info!("resumed");

            pixels = Some({
                let window_size = window.inner_size();
                let surface_texture =
                    SurfaceTexture::new(window_size.width, window_size.height, &window);
                Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap()
            });
        }

        if let Event::Suspended = event {
            pixels = None;
        }

        if let Some(pixels) = pixels.as_mut() {
            // Draw the current frame
            match event {
                Event::RedrawRequested(_) => {
                    world.draw(pixels.get_frame());
                    if pixels
                        .render()
                        .map_err(|e| error!("pixels.render() failed: {}", e))
                        .is_err()
                    {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }
                Event::MainEventsCleared => {
                    // Update internal state and request a redraw
                    world.update();
                    window.request_redraw();
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }

                Event::WindowEvent {
                    event: WindowEvent::Touch(touch),
                    ..
                } => {
                    if touch.phase == TouchPhase::Started {
                        // toggle software keyboard
                        soft_keyboard = !soft_keyboard;
                        show_soft_input(soft_keyboard);
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { input, .. },
                    ..
                } => {
                    log::info!("input: {:?}", input);
                }

                _ => (),
            }
        }
    });
}

impl World {
    /// Create a new `World` instance that can draw a moving box.
    fn new() -> Self {
        Self {
            box_x: 24,
            box_y: 16,
            velocity_x: 1,
            velocity_y: 1,
        }
    }

    /// Update the `World` internal state; bounce the box around the screen.
    fn update(&mut self) {
        if self.box_x <= 0 || self.box_x + BOX_SIZE > WIDTH as i16 {
            self.velocity_x *= -1;
        }
        if self.box_y <= 0 || self.box_y + BOX_SIZE > HEIGHT as i16 {
            self.velocity_y *= -1;
        }

        self.box_x += self.velocity_x;
        self.box_y += self.velocity_y;
    }

    /// Draw the `World` state to the frame buffer.
    ///
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % WIDTH as usize) as i16;
            let y = (i / WIDTH as usize) as i16;

            let inside_the_box = x >= self.box_x
                && x < self.box_x + BOX_SIZE
                && y >= self.box_y
                && y < self.box_y + BOX_SIZE;

            let rgba = if inside_the_box {
                [0x5e, 0x48, 0xe8, 0xff]
            } else {
                [0x48, 0xb2, 0xe8, 0xff]
            };

            pixel.copy_from_slice(&rgba);
        }
    }
}
