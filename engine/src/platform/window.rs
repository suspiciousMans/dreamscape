use std::sync::Arc;

use sdl2::video::{GLContext, GLProfile, Window as SdlWindow};
use sdl2::{EventPump, GameControllerSubsystem, Sdl, VideoSubsystem};

pub struct Platform {
    pub sdl: Sdl,
    pub video: VideoSubsystem,
    pub window: SdlWindow,
    // Held only to keep the GL context alive for the window's lifetime; dropping it
    // blacks the screen even though the window itself survives.
    _gl_context: GLContext,
    // Arc so consumers that need ownership (e.g. egui_glow::Painter) can share
    // it without cloning the underlying GL context, which glow doesn't support.
    pub gl: Arc<glow::Context>,
    pub event_pump: EventPump,
    // Held so opened `GameController`s stay valid; must outlive them.
    pub game_controller: GameControllerSubsystem,
}

impl Platform {
    pub fn new(title: &str, width: u32, height: u32) -> anyhow::Result<Self> {
        let sdl = sdl2::init().map_err(anyhow::Error::msg)?;
        let video = sdl.video().map_err(anyhow::Error::msg)?;

        // GL attributes must be set before window/context creation.
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_version(3, 3);
        gl_attr.set_double_buffer(true);
        gl_attr.set_depth_size(24);

        let window = video
            .window(title, width, height)
            .opengl()
            .resizable()
            .position_centered()
            .build()?;

        let gl_context = window.gl_create_context().map_err(anyhow::Error::msg)?;
        window.gl_make_current(&gl_context).map_err(anyhow::Error::msg)?;

        // Must load after the context is current.
        let gl = Arc::new(unsafe {
            glow::Context::from_loader_function(|s| video.gl_get_proc_address(s) as *const _)
        });

        if let Err(e) = video.gl_set_swap_interval(sdl2::video::SwapInterval::VSync) {
            log::warn!("vsync not available, falling back to immediate swap: {e}");
            video
                .gl_set_swap_interval(sdl2::video::SwapInterval::Immediate)
                .ok();
        }

        let event_pump = sdl.event_pump().map_err(anyhow::Error::msg)?;
        let game_controller = sdl.game_controller().map_err(anyhow::Error::msg)?;

        Ok(Self {
            sdl,
            video,
            window,
            _gl_context: gl_context,
            gl,
            event_pump,
            game_controller,
        })
    }

    pub fn drawable_size(&self) -> (u32, u32) {
        self.window.drawable_size()
    }

    pub fn swap_window(&self) {
        self.window.gl_swap_window();
    }
}
