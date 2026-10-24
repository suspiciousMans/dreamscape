//! Render-state caches for the world passes (WORLDGL perf stream).
//!
//! GL uniforms, the bound texture and the bound VAO keep their value until
//! changed, so a draw only needs to set what differs from the previous draw.
//! These helpers are pure (no GL) so the hit/miss logic is unit-tested; the
//! caller invalidates them at the start of every frame because egui (drawn
//! after the world) leaves its own state behind.

use engine::ecs::Entity;
use engine::glow;

/// Remembers the last value sent for one piece of GL state.
#[derive(Clone, Copy, Debug)]
pub struct Cached<T: Copy + PartialEq> {
    last: Option<T>,
}

impl<T: Copy + PartialEq> Default for Cached<T> {
    fn default() -> Self {
        Self { last: None }
    }
}

impl<T: Copy + PartialEq> Cached<T> {
    /// True when `v` differs from the last value (or none is known yet); the
    /// caller must then issue the GL call. Records `v`.
    pub fn needs_set(&mut self, v: T) -> bool {
        if self.last == Some(v) {
            false
        } else {
            self.last = Some(v);
            true
        }
    }

    #[cfg(test)]
    pub fn invalidate(&mut self) {
        self.last = None;
    }
}

/// Floats compared by bit pattern (so NaN does not defeat the cache and
/// -0.0 / 0.0 stay distinct, exactly like re-sending them would be).
pub type CachedF32 = Cached<u32>;

pub fn bits(v: f32) -> u32 {
    v.to_bits()
}

/// Per-draw state of the world shader plus counters for the profiler.
#[derive(Default)]
pub struct DrawState {
    pub lit: CachedF32,
    pub hero: CachedF32,
    pub melt: CachedF32,
    pub uv: CachedF32,
    pub texture: Cached<glow::Texture>,
    pub vao: Cached<glow::VertexArray>,
    pub stats: DrawStats,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawStats {
    pub draws: usize,
    pub uniform_calls: usize,
    pub uniform_skipped: usize,
    pub texture_binds: usize,
    pub texture_skipped: usize,
    pub vao_binds: usize,
    pub vao_skipped: usize,
}

impl DrawState {
    /// Forget everything. The frame builds a fresh `DrawState`, which is the
    /// same thing; this is for tests.
    #[cfg(test)]
    pub fn invalidate(&mut self) {
        self.lit.invalidate();
        self.hero.invalidate();
        self.melt.invalidate();
        self.uv.invalidate();
        self.texture.invalidate();
        self.vao.invalidate();
        self.stats = DrawStats::default();
    }

    /// Records a float uniform; returns whether it must be uploaded.
    pub fn float(&mut self, which: fn(&mut DrawState) -> &mut CachedF32, v: f32) -> bool {
        let set = which(self).needs_set(bits(v));
        if set {
            self.stats.uniform_calls += 1;
        } else {
            self.stats.uniform_skipped += 1;
        }
        set
    }

    pub fn texture_needs_bind(&mut self, t: glow::Texture) -> bool {
        let set = self.texture.needs_set(t);
        if set {
            self.stats.texture_binds += 1;
        } else {
            self.stats.texture_skipped += 1;
        }
        set
    }

    pub fn vao_needs_bind(&mut self, v: glow::VertexArray) -> bool {
        let set = self.vao.needs_set(v);
        if set {
            self.stats.vao_binds += 1;
        } else {
            self.stats.vao_skipped += 1;
        }
        set
    }
}

pub fn lit_of(s: &mut DrawState) -> &mut CachedF32 {
    &mut s.lit
}
pub fn hero_of(s: &mut DrawState) -> &mut CachedF32 {
    &mut s.hero
}
pub fn melt_of(s: &mut DrawState) -> &mut CachedF32 {
    &mut s.melt
}
pub fn uv_of(s: &mut DrawState) -> &mut CachedF32 {
    &mut s.uv
}

/// Every uniform location the world shader's frame setup and draws use,
/// looked up once per compiled program instead of ~30 string lookups a frame.
pub struct WorldLocs {
    pub program: glow::Program,
    pub frame: Vec<(&'static str, Option<glow::UniformLocation>)>,
    pub model: Option<glow::UniformLocation>,
    pub uv: Option<glow::UniformLocation>,
    pub melt: Option<glow::UniformLocation>,
    pub lit: Option<glow::UniformLocation>,
    pub hero: Option<glow::UniformLocation>,
    pub ghost: Option<glow::UniformLocation>,
    pub outline: Option<glow::UniformLocation>,
}

/// Names set once per frame (see `render`).
pub const FRAME_UNIFORMS: [&str; 20] = [
    "uView", "uProj", "uLightDir", "uAmbientColor", "uLightingMode", "uVertexSnapAmount",
    "uFogStart", "uFogEnd", "uFogColor", "uPointLightCount", "uTex", "uTime", "uStrangeness",
    "uBreathe", "uPulse", "uGlow", "uPlayer", "uSight", "uSightFade", "uDark"
];

impl WorldLocs {
    pub fn new(gl: &glow::Context, program: glow::Program) -> Self {
        use engine::glow::HasContext;
        let get = |n: &str| unsafe { gl.get_uniform_location(program, n) };
        Self {
            program,
            frame: FRAME_UNIFORMS.iter().map(|n| (*n, get(n))).collect(),
            model: get("uModel"),
            uv: get("uUVScale"),
            melt: get("uMelt"),
            lit: get("uLit"),
            hero: get("uHero"),
            ghost: get("uGhost"),
            outline: get("uOutline"),
        }
    }

    /// Cached location of a per-frame uniform.
    pub fn frame(&self, name: &str) -> Option<&glow::UniformLocation> {
        self.frame
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, l)| l.as_ref())
    }
}

/// Entities each later pass wants, collected during the opaque pass so the
/// world is queried once per frame instead of four times. The Vecs are kept
/// between frames (cleared, not freed). Order is the world's query order, which
/// is the order the old per-pass queries drew in.
#[derive(Default)]
pub struct PassLists {
    /// Heroes: outline shell (pass 1) and silhouette (pass 3).
    pub heroes: Vec<Entity>,
    /// Translucent water (pass 2).
    pub water: Vec<Entity>,
}

impl PassLists {
    pub fn clear(&mut self) {
        self.heroes.clear();
        self.water.clear();
    }

    /// Routes one drawable entity. Returns true when it is drawn in the opaque
    /// pass (not water).
    pub fn route(&mut self, e: Entity, is_water: bool, is_hero: bool) -> bool {
        if is_hero {
            self.heroes.push(e);
        }
        if is_water {
            self.water.push(e);
        }
        !is_water
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::ecs::World;

    #[test]
    fn cache_skips_repeats_and_sets_changes() {
        let mut c = CachedF32::default();
        assert!(c.needs_set(bits(1.0)));
        assert!(!c.needs_set(bits(1.0)));
        assert!(c.needs_set(bits(0.0)));
        assert!(c.needs_set(bits(-0.0)), "-0.0 is a different bit pattern");
        assert!(c.needs_set(bits(f32::NAN)));
        assert!(!c.needs_set(bits(f32::NAN)), "NaN repeats hit the cache");
    }

    #[test]
    fn invalidate_forces_next_set() {
        let mut s = DrawState::default();
        assert!(s.float(lit_of, 1.0));
        assert!(!s.float(lit_of, 1.0));
        assert!(s.float(hero_of, 1.0), "separate uniforms are independent");
        s.invalidate();
        assert!(s.float(lit_of, 1.0));
        assert_eq!(s.stats.uniform_skipped, 0, "stats reset on invalidate");
    }

    #[test]
    fn counts_hits_and_misses() {
        let mut s = DrawState::default();
        for v in [0.0, 0.0, 0.0, 1.0, 1.0] {
            s.float(melt_of, v);
        }
        assert_eq!(s.stats.uniform_calls, 2);
        assert_eq!(s.stats.uniform_skipped, 3);
    }

    #[test]
    fn routing_keeps_query_order_and_splits_passes() {
        let mut w = World::new();
        let es: Vec<Entity> = (0..4).map(|i| w.spawn((i,))).collect();
        let mut l = PassLists::default();
        assert!(l.route(es[0], false, false));
        assert!(l.route(es[1], false, true));
        assert!(!l.route(es[2], true, false));
        assert!(!l.route(es[3], true, true));
        assert_eq!(l.heroes, vec![es[1], es[3]]);
        assert_eq!(l.water, vec![es[2], es[3]]);
        let cap = l.heroes.capacity();
        l.clear();
        assert!(l.heroes.is_empty() && l.water.is_empty());
        assert_eq!(l.heroes.capacity(), cap, "buffers survive clear");
    }
}
