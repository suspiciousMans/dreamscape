//! === perf === Builds a dream's procedural textures on several threads.
//! `load_dream` needs ~7 textures (floor, wall, prop, shard, portal, wake,
//! enemy) at ~0.6 ms each, one after the other on the main thread. They are
//! pure functions of their `TexSpec`, so they are made side by side; the GL
//! uploads stay on the main thread. The bytes are identical to `spec.rgba()`.
use crate::dream::TexSpec;

/// `spec.rgba()` for each spec, in order (parallel where threads exist).
pub fn generate_all(specs: &[&TexSpec]) -> Vec<Vec<u8>> {
    #[cfg(not(target_os = "emscripten"))]
    {
        if specs.len() > 1 {
            return std::thread::scope(|s| {
                let jobs: Vec<_> = specs.iter().map(|&sp| s.spawn(move || sp.rgba())).collect();
                jobs.into_iter()
                    .zip(specs)
                    .map(|(j, sp)| j.join().unwrap_or_else(|_| sp.rgba()))
                    .collect()
            });
        }
    }
    specs.iter().map(|s| s.rgba()).collect()
}

/// Pixels made ahead of the uploads, found by spec.
#[derive(Default)]
pub struct Prefetched(Vec<(TexSpec, Vec<u8>)>);

impl Prefetched {
    pub fn fill(&mut self, specs: &[&TexSpec]) {
        let px = generate_all(specs);
        self.0 = specs.iter().map(|s| (*s).clone()).zip(px).collect();
    }

    /// The pixels for `spec`: the prefetched copy, or made now.
    pub fn pixels(&self, spec: &TexSpec) -> Vec<u8> {
        match self.0.iter().find(|(s, _)| s == spec) {
            Some((_, px)) => px.clone(),
            None => spec.rgba(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::{DreamTheme, ALL_THEMES};

    #[test]
    fn parallel_pixels_equal_serial_pixels() {
        let d = crate::dream::generate_with(
            DreamTheme::Garden, 3, 4, None, true, Default::default(), Default::default(),
        );
        let specs = [&d.surfaces.floor, &d.surfaces.wall, &d.surfaces.prop, &d.surfaces.shard, &d.surfaces.enemy];
        let par = generate_all(&specs);
        for (s, p) in specs.iter().zip(&par) {
            assert_eq!(&s.rgba(), p);
        }
        let mut pre = Prefetched::default();
        pre.fill(&specs);
        assert_eq!(pre.pixels(&d.surfaces.wall), d.surfaces.wall.rgba());
        let other = crate::dream::portal_surface(ALL_THEMES[1], 9);
        assert_eq!(pre.pixels(&other), other.rgba(), "a miss falls back to making it now");
    }
}
