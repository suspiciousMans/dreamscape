//! White Dissolve: the way behind you dissolves while you move and comes back
//! when you stand still. Pure state; main.rs mirrors it onto floor tiles.

use crate::gameplay::CELL;
use engine::glam::Vec3;

pub const FADE: f32 = 1.2;
pub const REFORM_AFTER: f32 = 0.8;
pub const STILL_SPEED: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tile {
    Solid,
    Fading(f32),
    Gone,
}

#[derive(Clone, Debug)]
pub struct Dissolve {
    pub tiles: Vec<(Vec3, Tile)>,
    still: f32,
    current: Option<usize>,
}

impl Dissolve {
    pub fn new(cells: Vec<Vec3>) -> Self {
        Self {
            tiles: cells.into_iter().map(|c| (c, Tile::Solid)).collect(),
            still: 0.0,
            current: None,
        }
    }

    pub fn tile_at(&self, p: Vec3) -> Option<usize> {
        self.tiles
            .iter()
            .position(|(c, _)| (c.x - p.x).abs() <= CELL * 0.5 && (c.z - p.z).abs() <= CELL * 0.5)
    }

    pub fn update(&mut self, player: Vec3, speed: f32, dt: f32) {
        let moving = speed >= STILL_SPEED;
        self.still = if moving { 0.0 } else { self.still + dt };
        if self.still >= REFORM_AFTER {
            self.reset();
        }
        for (_, t) in &mut self.tiles {
            if let Tile::Fading(left) = *t {
                *t = if left - dt <= 0.0 {
                    Tile::Gone
                } else {
                    Tile::Fading(left - dt)
                };
            }
        }
        let here = self.tile_at(player);
        if here != self.current {
            if let Some(prev) = self.current {
                if moving && self.tiles[prev].1 == Tile::Solid {
                    self.tiles[prev].1 = Tile::Fading(FADE);
                }
            }
            self.current = here;
        }
    }

    pub fn reset(&mut self) {
        for (_, t) in &mut self.tiles {
            *t = Tile::Solid;
        }
    }

    pub fn solid(&self, i: usize) -> bool {
        self.tiles[i].1 != Tile::Gone
    }

    /// 1 = fully there, 0 = gone.
    pub fn opacity(&self, i: usize) -> f32 {
        match self.tiles[i].1 {
            Tile::Solid => 1.0,
            Tile::Fading(left) => left / FADE,
            Tile::Gone => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::MOVE_SPEED;
    const DT: f32 = 1.0 / 60.0;

    fn line(n: usize) -> Vec<Vec3> {
        (0..n)
            .map(|k| Vec3::new(0.0, 0.0, k as f32 * CELL))
            .collect()
    }

    /// Walks waypoints; a careful walker stops whenever the next tile isn't solid.
    fn walk(d: &mut Dissolve, path: &[Vec3], careful: bool) -> Result<(), String> {
        let mut pos = path[0];
        for &goal in &path[1..] {
            let mut t = 0.0;
            while pos.distance(goal) > 1e-3 {
                t += DT;
                if t > 30.0 {
                    return Err("stuck".into());
                }
                let next = d.tile_at(goal).expect("waypoint on a tile");
                let wait = careful && !matches!(d.tiles[next].1, Tile::Solid);
                let speed = if wait { 0.0 } else { MOVE_SPEED };
                if !wait {
                    let to = goal - pos;
                    pos += to.normalize_or_zero() * (MOVE_SPEED * DT).min(to.length());
                }
                d.update(pos, speed, DT);
                if let Some(i) = d.tile_at(pos) {
                    if !d.solid(i) {
                        return Err(format!("stood on a gone tile at {pos:?}"));
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn walking_on_is_always_safe() {
        let cells = line(12);
        assert!(walk(&mut Dissolve::new(cells.clone()), &cells, false).is_ok());
    }

    #[test]
    fn turning_straight_back_drops_you_and_stopping_saves_you() {
        let cells = line(12);
        let there_and_back: Vec<Vec3> = cells
            .iter()
            .chain(cells.iter().rev().skip(1))
            .copied()
            .collect();
        assert!(
            walk(&mut Dissolve::new(cells.clone()), &there_and_back, false).is_err(),
            "no danger at all?"
        );
        assert!(
            walk(&mut Dissolve::new(cells), &there_and_back, true).is_ok(),
            "a careful walker fell"
        );
    }

    #[test]
    fn a_tile_takes_fade_seconds_to_go() {
        let mut d = Dissolve::new(line(3));
        d.update(Vec3::ZERO, MOVE_SPEED, DT);
        d.update(Vec3::Z * CELL, MOVE_SPEED, DT);
        let mut t = 0.0;
        while d.solid(0) {
            d.update(Vec3::Z * CELL, MOVE_SPEED, DT);
            t += DT;
        }
        assert!((t - FADE).abs() < 3.0 * DT, "took {t}");
    }
}
