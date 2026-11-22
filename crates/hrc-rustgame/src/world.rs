//! عالم مكعبات بسيط: تضاريس من ضجيج قيمي (value noise)، chunks بحجم 16×16×64، وبناء mesh بإخفاء الأوجه الداخلية.
//! لا علاقة له بعوالم Minecraft ولا بصيغتها؛ كل شيء هنا مكتوب من الصفر.
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;

pub const CH: i32 = 16;
pub const HEIGHT: i32 = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Block {
    Air,
    Grass,
    Dirt,
    Stone,
    Sand,
    Snow,
}

impl Block {
    pub fn color(self) -> [f32; 3] {
        match self {
            Block::Air => [0.0; 3],
            Block::Grass => [0.36, 0.62, 0.22],
            Block::Dirt => [0.45, 0.32, 0.20],
            Block::Stone => [0.50, 0.50, 0.52],
            Block::Sand => [0.86, 0.80, 0.55],
            Block::Snow => [0.95, 0.96, 0.98],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Default, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn append(&mut self, other: Mesh) {
        let base = self.vertices.len() as u32;
        self.vertices.extend(other.vertices);
        self.indices.extend(other.indices.into_iter().map(|i| i + base));
    }
    pub fn faces(&self) -> usize {
        self.indices.len() / 6
    }
}

fn hash(x: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (z as u32).wrapping_mul(0x85EB_CA77) ^ seed.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 16_777_216.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let (x0, z0) = (x.floor(), z.floor());
    let (fx, fz) = (smooth(x - x0), smooth(z - z0));
    let (ix, iz) = (x0 as i32, z0 as i32);
    let a = hash(ix, iz, seed);
    let b = hash(ix + 1, iz, seed);
    let c = hash(ix, iz + 1, seed);
    let d = hash(ix + 1, iz + 1, seed);
    let top = a + (b - a) * fx;
    let bot = c + (d - c) * fx;
    top + (bot - top) * fz
}

/// ارتفاع سطح الأرض عند عمود (x, z): 4 طبقات ضجيج متدرجة.
pub fn height_at(x: i32, z: i32, seed: u32) -> i32 {
    let (mut amp, mut freq, mut sum, mut norm) = (1.0f32, 1.0f32 / 48.0, 0.0f32, 0.0f32);
    for o in 0..4u32 {
        sum += value_noise(x as f32 * freq, z as f32 * freq, seed.wrapping_add(o * 101)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    let h = 8.0 + (sum / norm) * 40.0;
    (h as i32).clamp(2, HEIGHT - 2)
}

pub struct Chunk {
    blocks: Vec<Block>,
}

impl Chunk {
    fn idx(x: i32, y: i32, z: i32) -> usize {
        ((y * CH + z) * CH + x) as usize
    }
    fn empty() -> Chunk {
        Chunk { blocks: vec![Block::Air; (CH * CH * HEIGHT) as usize] }
    }
    fn generate(cx: i32, cz: i32, seed: u32) -> Chunk {
        let mut c = Chunk::empty();
        for lz in 0..CH {
            for lx in 0..CH {
                let h = height_at(cx * CH + lx, cz * CH + lz, seed);
                for y in 0..=h {
                    let b = if y == h {
                        if h <= 16 {
                            Block::Sand
                        } else if h >= 34 {
                            Block::Snow
                        } else {
                            Block::Grass
                        }
                    } else if y >= h - 3 {
                        Block::Dirt
                    } else {
                        Block::Stone
                    };
                    c.blocks[Chunk::idx(lx, y, lz)] = b;
                }
            }
        }
        c
    }
}

pub struct World {
    pub seed: u32,
    chunks: HashMap<(i32, i32), Chunk>,
}

/// (اتجاه الجار، ظل الوجه، أركان الوجه بترتيب CCW من الخارج)
type Face = ([i32; 3], f32, [[f32; 3]; 4]);
const FACES: [Face; 6] = [
    ([1, 0, 0], 0.8, [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]]),
    ([-1, 0, 0], 0.8, [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]]),
    ([0, 1, 0], 1.0, [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]]),
    ([0, -1, 0], 0.5, [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]]),
    ([0, 0, 1], 0.7, [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]]),
    ([0, 0, -1], 0.7, [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]]),
];

impl World {
    pub fn new(seed: u32) -> World {
        World { seed, chunks: HashMap::new() }
    }

    pub fn generate_around(&mut self, cx: i32, cz: i32, radius: i32) {
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                let key = (cx + dx, cz + dz);
                let seed = self.seed;
                self.chunks.entry(key).or_insert_with(|| Chunk::generate(key.0, key.1, seed));
            }
        }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn block(&self, x: i32, y: i32, z: i32) -> Block {
        if y < 0 {
            return Block::Stone; // القاع: لا نرسم أوجهه
        }
        if y >= HEIGHT {
            return Block::Air;
        }
        match self.chunks.get(&(x.div_euclid(CH), z.div_euclid(CH))) {
            Some(c) => c.blocks[Chunk::idx(x.rem_euclid(CH), y, z.rem_euclid(CH))],
            None => Block::Air,
        }
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, b: Block) {
        if !(0..HEIGHT).contains(&y) {
            return;
        }
        let c = self.chunks.entry((x.div_euclid(CH), z.div_euclid(CH))).or_insert_with(Chunk::empty);
        c.blocks[Chunk::idx(x.rem_euclid(CH), y, z.rem_euclid(CH))] = b;
    }

    pub fn mesh_chunk(&self, cx: i32, cz: i32) -> Mesh {
        let mut m = Mesh::default();
        let Some(chunk) = self.chunks.get(&(cx, cz)) else { return m };
        for y in 0..HEIGHT {
            for lz in 0..CH {
                for lx in 0..CH {
                    let b = chunk.blocks[Chunk::idx(lx, y, lz)];
                    if b == Block::Air {
                        continue;
                    }
                    let (wx, wz) = (cx * CH + lx, cz * CH + lz);
                    for (dir, shade, corners) in FACES.iter() {
                        let (nx, ny, nz) = (lx + dir[0], y + dir[1], lz + dir[2]);
                        let neighbor = if (0..CH).contains(&nx) && (0..HEIGHT).contains(&ny) && (0..CH).contains(&nz) {
                            chunk.blocks[Chunk::idx(nx, ny, nz)]
                        } else {
                            self.block(wx + dir[0], y + dir[1], wz + dir[2])
                        };
                        if neighbor != Block::Air {
                            continue;
                        }
                        let col = b.color();
                        let color = [col[0] * shade, col[1] * shade, col[2] * shade];
                        let base = m.vertices.len() as u32;
                        for c in corners {
                            m.vertices.push(Vertex { pos: [wx as f32 + c[0], y as f32 + c[1], wz as f32 + c[2]], color });
                        }
                        m.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                    }
                }
            }
        }
        m
    }

    pub fn mesh_all(&self) -> Mesh {
        let mut all = Mesh::default();
        let mut keys: Vec<_> = self.chunks.keys().copied().collect();
        keys.sort();
        for (cx, cz) in keys {
            all.append(self.mesh_chunk(cx, cz));
        }
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_is_deterministic_and_bounded() {
        for (x, z) in [(0, 0), (5, -9), (-300, 400), (1000, 1000)] {
            let h = height_at(x, z, 42);
            assert_eq!(h, height_at(x, z, 42));
            assert!((2..HEIGHT - 1).contains(&h), "{h}");
        }
        let a: Vec<i32> = (0..64).map(|x| height_at(x, 0, 1)).collect();
        let b: Vec<i32> = (0..64).map(|x| height_at(x, 0, 2)).collect();
        assert_ne!(a, b, "different seeds must give different terrain");
    }

    #[test]
    fn single_block_has_six_faces() {
        let mut w = World::new(1);
        w.set_block(3, 10, 3, Block::Stone);
        let m = w.mesh_chunk(0, 0);
        assert_eq!(m.faces(), 6);
        assert_eq!(m.vertices.len(), 24);
        assert_eq!(m.indices.len(), 36);
    }

    #[test]
    fn touching_blocks_hide_shared_face() {
        let mut w = World::new(1);
        w.set_block(3, 10, 3, Block::Stone);
        w.set_block(4, 10, 3, Block::Stone);
        assert_eq!(w.mesh_chunk(0, 0).faces(), 10); // 2 مكعبات = 12 وجه − وجهان داخليان
    }

    #[test]
    fn faces_across_chunk_border_are_culled() {
        let mut w = World::new(1);
        w.set_block(15, 10, 3, Block::Stone);
        w.set_block(16, 10, 3, Block::Stone); // في chunk مجاور
        assert_eq!(w.mesh_chunk(0, 0).faces(), 5);
        assert_eq!(w.mesh_chunk(1, 0).faces(), 5);
    }

    #[test]
    fn bottom_layer_hides_its_floor() {
        let mut w = World::new(1);
        w.set_block(0, 0, 0, Block::Stone);
        assert_eq!(w.mesh_chunk(0, 0).faces(), 5);
    }

    #[test]
    fn generated_world_has_solid_ground_and_valid_mesh() {
        let mut w = World::new(1337);
        w.generate_around(0, 0, 1);
        assert_eq!(w.chunk_count(), 9);
        let h = height_at(8, 8, 1337);
        assert_ne!(w.block(8, h, 8), Block::Air);
        assert_eq!(w.block(8, h + 1, 8), Block::Air);
        assert_eq!(w.block(8, 0, 8), Block::Stone);
        let m = w.mesh_all();
        assert!(m.faces() > 500);
        assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.len()));
    }

    #[test]
    fn negative_coordinates_work() {
        let mut w = World::new(1);
        w.generate_around(0, 0, 1);
        assert_ne!(w.block(-1, height_at(-1, -1, 1), -1), Block::Air);
    }
}
