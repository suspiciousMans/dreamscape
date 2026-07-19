use glam::Vec3;

use crate::ecs::Transform;
use crate::physics::{Collider, ColliderShape, RigidBody};

/// Minimum Y half-extent (or radius) a collider needs to count as an
/// obstruction when baking a `NavGrid` — keeps a paper-thin floor plane
/// (Y half-extent ~0.1) from blocking every cell while pillars/walls,
/// being much taller, still block correctly.
const MIN_OBSTRUCTION_HEIGHT: f32 = 0.3;
/// Extra cells of clearance baked in beyond the tightest bounding box of
/// static geometry, so a character never needs to path along the very edge
/// of the grid.
const BOUNDS_MARGIN_CELLS: f32 = 4.0;

/// A baked walkability grid in the XZ plane, used for `find_path`. Baked
/// once per level load (see `Sandbox::apply_level`) from every static
/// (no `RigidBody`), non-trigger `Collider` in the world — level layouts
/// here are static rooms, not destructible/moving geometry, so no dynamic
/// re-baking mid-level is needed.
pub struct NavGrid {
    origin_x: f32,
    origin_z: f32,
    cell_size: f32,
    width: usize,
    depth: usize,
    walkable: Vec<bool>,
}

impl NavGrid {
    pub fn bake(world: &hecs::World, cell_size: f32) -> Self {
        let obstacles: Vec<(f32, f32, f32, f32)> = world
            .query::<(&Transform, &Collider)>()
            .without::<&RigidBody>()
            .iter()
            .filter(|(_, (_, collider))| !collider.is_trigger)
            .filter_map(|(_, (transform, collider))| {
                let (half_x, half_y, half_z) = match collider.shape {
                    ColliderShape::Aabb { half_extents } => (half_extents.x, half_extents.y, half_extents.z),
                    ColliderShape::Sphere { radius } => (radius, radius, radius),
                };
                if half_y < MIN_OBSTRUCTION_HEIGHT {
                    return None;
                }
                let position = transform.position;
                Some((position.x - half_x, position.x + half_x, position.z - half_z, position.z + half_z))
            })
            .collect();

        if obstacles.is_empty() {
            return Self { origin_x: 0.0, origin_z: 0.0, cell_size, width: 0, depth: 0, walkable: Vec::new() };
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;
        for &(ox_min, ox_max, oz_min, oz_max) in &obstacles {
            min_x = min_x.min(ox_min);
            max_x = max_x.max(ox_max);
            min_z = min_z.min(oz_min);
            max_z = max_z.max(oz_max);
        }

        let margin = BOUNDS_MARGIN_CELLS * cell_size;
        let origin_x = min_x - margin;
        let origin_z = min_z - margin;
        let width = (((max_x - min_x) + margin * 2.0) / cell_size).ceil().max(1.0) as usize;
        let depth = (((max_z - min_z) + margin * 2.0) / cell_size).ceil().max(1.0) as usize;

        let mut walkable = vec![true; width * depth];
        for cz in 0..depth {
            for cx in 0..width {
                let cell_min_x = origin_x + cx as f32 * cell_size;
                let cell_max_x = cell_min_x + cell_size;
                let cell_min_z = origin_z + cz as f32 * cell_size;
                let cell_max_z = cell_min_z + cell_size;
                let blocked = obstacles.iter().any(|&(ox_min, ox_max, oz_min, oz_max)| {
                    cell_min_x < ox_max && cell_max_x > ox_min && cell_min_z < oz_max && cell_max_z > oz_min
                });
                if blocked {
                    walkable[cz * width + cx] = false;
                }
            }
        }

        Self { origin_x, origin_z, cell_size, width, depth, walkable }
    }

    fn cell_of(&self, position: Vec3) -> Option<(usize, usize)> {
        if self.width == 0 || self.depth == 0 {
            return None;
        }
        let cx = ((position.x - self.origin_x) / self.cell_size).floor();
        let cz = ((position.z - self.origin_z) / self.cell_size).floor();
        if cx < 0.0 || cz < 0.0 {
            return None;
        }
        let (cx, cz) = (cx as usize, cz as usize);
        (cx < self.width && cz < self.depth).then_some((cx, cz))
    }

    fn is_walkable(&self, cx: usize, cz: usize) -> bool {
        self.walkable.get(cz * self.width + cx).copied().unwrap_or(false)
    }

    fn cell_center(&self, cell: (usize, usize), y: f32) -> Vec3 {
        Vec3::new(
            self.origin_x + (cell.0 as f32 + 0.5) * self.cell_size,
            y,
            self.origin_z + (cell.1 as f32 + 0.5) * self.cell_size,
        )
    }

    /// Grid-based A* (8-directional, octile heuristic, no cutting a corner
    /// diagonally through two blocked orthogonal cells) from `start` to
    /// `goal`, both world-space. Returns waypoint centers in world space
    /// (Y taken from `goal`), or `None` if unreachable or out of bounds.
    pub fn find_path(&self, start: Vec3, goal: Vec3) -> Option<Vec<Vec3>> {
        let start_cell = self.cell_of(start)?;
        let goal_cell = self.cell_of(goal)?;
        if !self.is_walkable(goal_cell.0, goal_cell.1) {
            return None;
        }
        if start_cell == goal_cell {
            return Some(vec![goal]);
        }

        let index = |cell: (usize, usize)| cell.1 * self.width + cell.0;
        let octile = |a: (usize, usize), b: (usize, usize)| {
            let dx = (a.0 as f32 - b.0 as f32).abs();
            let dz = (a.1 as f32 - b.1 as f32).abs();
            dx.max(dz) + (std::f32::consts::SQRT_2 - 1.0) * dx.min(dz)
        };

        let mut open = std::collections::BinaryHeap::new();
        let mut g_score = vec![f32::MAX; self.width * self.depth];
        let mut came_from: Vec<Option<(usize, usize)>> = vec![None; self.width * self.depth];
        let mut visited = vec![false; self.width * self.depth];
        g_score[index(start_cell)] = 0.0;
        open.push(ScoredCell { cost: octile(start_cell, goal_cell), cell: start_cell });

        while let Some(ScoredCell { cell, .. }) = open.pop() {
            if visited[index(cell)] {
                continue;
            }
            visited[index(cell)] = true;
            if cell == goal_cell {
                break;
            }
            for dz in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dz == 0 {
                        continue;
                    }
                    let nx = cell.0 as i32 + dx;
                    let nz = cell.1 as i32 + dz;
                    if nx < 0 || nz < 0 || nx as usize >= self.width || nz as usize >= self.depth {
                        continue;
                    }
                    let neighbor = (nx as usize, nz as usize);
                    if !self.is_walkable(neighbor.0, neighbor.1) {
                        continue;
                    }
                    if dx != 0 && dz != 0 {
                        let horizontal_open = self.is_walkable(neighbor.0, cell.1);
                        let vertical_open = self.is_walkable(cell.0, neighbor.1);
                        if !horizontal_open || !vertical_open {
                            continue;
                        }
                    }
                    let step_cost = if dx != 0 && dz != 0 { std::f32::consts::SQRT_2 } else { 1.0 };
                    let tentative = g_score[index(cell)] + step_cost;
                    if tentative < g_score[index(neighbor)] {
                        g_score[index(neighbor)] = tentative;
                        came_from[index(neighbor)] = Some(cell);
                        open.push(ScoredCell { cost: tentative + octile(neighbor, goal_cell), cell: neighbor });
                    }
                }
            }
        }

        if !visited[index(goal_cell)] {
            return None;
        }

        let mut path_cells = vec![goal_cell];
        let mut current = goal_cell;
        while let Some(prev) = came_from[index(current)] {
            path_cells.push(prev);
            current = prev;
        }
        path_cells.reverse();

        Some(path_cells.into_iter().map(|cell| self.cell_center(cell, goal.y)).collect())
    }
}

#[derive(Copy, Clone, PartialEq)]
struct ScoredCell {
    cost: f32,
    cell: (usize, usize),
}
impl Eq for ScoredCell {}
impl Ord for ScoredCell {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reversed so `BinaryHeap` (a max-heap) pops the lowest cost first.
        other.cost.partial_cmp(&self.cost).unwrap_or(std::cmp::Ordering::Equal)
    }
}
impl PartialOrd for ScoredCell {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall(world: &mut hecs::World, position: Vec3, half_extents: Vec3) {
        world.spawn((
            Transform { position, rotation: glam::Quat::IDENTITY, scale: Vec3::ONE },
            Collider { shape: ColliderShape::Aabb { half_extents }, is_trigger: false },
        ));
    }

    fn bordered_room(world: &mut hecs::World) {
        wall(world, Vec3::new(-6.0, 0.0, 0.0), Vec3::new(0.5, 1.0, 6.0));
        wall(world, Vec3::new(6.0, 0.0, 0.0), Vec3::new(0.5, 1.0, 6.0));
        wall(world, Vec3::new(0.0, 0.0, -6.0), Vec3::new(6.0, 1.0, 0.5));
        wall(world, Vec3::new(0.0, 0.0, 6.0), Vec3::new(6.0, 1.0, 0.5));
    }

    #[test]
    fn finds_a_direct_path_in_an_open_room() {
        let mut world = hecs::World::new();
        bordered_room(&mut world);

        let grid = NavGrid::bake(&world, 1.0);
        let path = grid
            .find_path(Vec3::new(-4.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 0.0))
            .expect("path should exist in an open room");
        assert!(path.len() >= 2);
        assert!(path.last().unwrap().distance(Vec3::new(4.0, 0.0, 0.0)) < 1.5);
    }

    #[test]
    fn a_thin_floor_never_blocks_the_grid() {
        let mut world = hecs::World::new();
        bordered_room(&mut world);
        // Half-extent 0.1 < MIN_OBSTRUCTION_HEIGHT — must not count as an
        // obstruction, or every cell in the room would be unwalkable.
        wall(&mut world, Vec3::new(0.0, -0.5, 0.0), Vec3::new(6.0, 0.1, 6.0));

        let grid = NavGrid::bake(&world, 1.0);
        assert!(grid.find_path(Vec3::new(-4.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 0.0)).is_some());
    }

    #[test]
    fn routes_around_a_dividing_wall() {
        let mut world = hecs::World::new();
        bordered_room(&mut world);
        // A dividing wall down the middle with a gap on the +x side, forcing
        // a detour rather than a straight line from one side to the other.
        wall(&mut world, Vec3::new(-1.5, 0.0, 0.0), Vec3::new(4.5, 1.0, 0.5));

        let grid = NavGrid::bake(&world, 1.0);
        let path = grid
            .find_path(Vec3::new(-4.0, 0.0, -3.0), Vec3::new(-4.0, 0.0, 3.0))
            .expect("path should route around the dividing wall's open end");
        assert!(path.iter().any(|p| p.x > 0.0), "path should detour through the wall's gap near x=4");
    }

    #[test]
    fn unreachable_goal_returns_none() {
        let mut world = hecs::World::new();
        bordered_room(&mut world);
        // A wall spanning the full width, sealing the room in half with no gap.
        wall(&mut world, Vec3::new(0.0, 0.0, 0.0), Vec3::new(6.0, 1.0, 0.5));

        let grid = NavGrid::bake(&world, 1.0);
        assert!(grid.find_path(Vec3::new(-4.0, 0.0, -3.0), Vec3::new(-4.0, 0.0, 3.0)).is_none());
    }
}
