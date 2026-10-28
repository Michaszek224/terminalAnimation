use std::f32::consts::{PI, TAU};

use clap::ValueEnum;

use crate::math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ShapeKind {
    Cube,
    Pyramid,
    Octahedron,
    Dodecahedron,
    Icosahedron,
    Sphere,
    Torus,
    Tesseract,
}

impl ShapeKind {
    pub fn build(self) -> Model {
        match self {
            ShapeKind::Cube => Model::Mesh(cube()),
            ShapeKind::Pyramid => Model::Mesh(pyramid()),
            ShapeKind::Octahedron => Model::Mesh(octahedron()),
            ShapeKind::Dodecahedron => Model::Mesh(dodecahedron()),
            ShapeKind::Icosahedron => {
                let (verts, faces) = icosahedron_raw();
                Model::Mesh(Mesh::flat(verts, faces))
            }
            ShapeKind::Sphere => Model::Mesh(sphere(12, 20)),
            ShapeKind::Torus => Model::Mesh(torus(28, 14, 0.42)),
            ShapeKind::Tesseract => Model::Tesseract(Tesseract::new()),
        }
    }
}

pub enum Model {
    Mesh(Mesh),
    Tesseract(Tesseract),
}

/// Triangle corner = (vertex index, normal index).
pub type Tri = [(usize, usize); 3];

/// Shape scaled to fit the unit sphere, ready for both fill and wireframe rendering.
pub struct Mesh {
    pub verts: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub tris: Vec<Tri>,
    pub edges: Vec<(usize, usize)>,
}

impl Mesh {
    /// Convex polyhedron with flat-shaded faces. Faces are vertex loops in any winding.
    fn flat(verts: Vec<Vec3>, faces: Vec<Vec<usize>>) -> Mesh {
        let verts = fit_unit_sphere(verts);
        let mut normals = Vec::with_capacity(faces.len());
        let mut tris = Vec::new();
        for (fi, face) in faces.iter().enumerate() {
            let centroid = face.iter().fold(Vec3::ZERO, |acc, &i| acc + verts[i]) / face.len() as f32;
            let (a, b, c) = (verts[face[0]], verts[face[1]], verts[face[2]]);
            let mut n = (b - a).cross(c - a).normalize();
            if n.dot(centroid) < 0.0 {
                n = -n;
            }
            normals.push(n);
            for k in 1..face.len() - 1 {
                tris.push([(face[0], fi), (face[k], fi), (face[k + 1], fi)]);
            }
        }
        Mesh { edges: edges_of(&faces), verts, normals, tris }
    }

    /// Curved surface with one normal per vertex (smooth shading).
    fn smooth(verts: Vec<Vec3>, normals: Vec<Vec3>, faces: Vec<Vec<usize>>) -> Mesh {
        let mut tris = Vec::new();
        for face in &faces {
            for k in 1..face.len() - 1 {
                tris.push([(face[0], face[0]), (face[k], face[k]), (face[k + 1], face[k + 1])]);
            }
        }
        Mesh { edges: edges_of(&faces), verts, normals, tris }
    }
}

fn fit_unit_sphere(verts: Vec<Vec3>) -> Vec<Vec3> {
    let center = verts.iter().fold(Vec3::ZERO, |acc, &v| acc + v) / verts.len() as f32;
    let radius = verts.iter().map(|&v| (v - center).length()).fold(0.0, f32::max);
    verts.into_iter().map(|v| (v - center) / radius).collect()
}

fn edges_of(faces: &[Vec<usize>]) -> Vec<(usize, usize)> {
    let mut edges: Vec<(usize, usize)> = faces
        .iter()
        .flat_map(|f| (0..f.len()).map(move |k| (f[k], f[(k + 1) % f.len()])))
        .map(|(a, b)| (a.min(b), a.max(b)))
        .collect();
    edges.sort_unstable();
    edges.dedup();
    edges
}

fn cube() -> Mesh {
    // Vertex index bits: x = bit 0, y = bit 1, z = bit 2.
    let verts = (0..8)
        .map(|i| {
            let s = |bit: usize| if i >> bit & 1 == 1 { 1.0 } else { -1.0 };
            Vec3::new(s(0), s(1), s(2))
        })
        .collect();
    let faces = vec![
        vec![0, 2, 6, 4],
        vec![1, 3, 7, 5],
        vec![0, 1, 5, 4],
        vec![2, 3, 7, 6],
        vec![0, 1, 3, 2],
        vec![4, 5, 7, 6],
    ];
    Mesh::flat(verts, faces)
}

fn pyramid() -> Mesh {
    let y = -0.8;
    let verts = vec![
        Vec3::new(-1.0, y, -1.0),
        Vec3::new(1.0, y, -1.0),
        Vec3::new(1.0, y, 1.0),
        Vec3::new(-1.0, y, 1.0),
        Vec3::new(0.0, 1.2, 0.0),
    ];
    let faces = vec![vec![0, 1, 2, 3], vec![0, 1, 4], vec![1, 2, 4], vec![2, 3, 4], vec![3, 0, 4]];
    Mesh::flat(verts, faces)
}

fn octahedron() -> Mesh {
    let verts = vec![
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ];
    let mut faces = Vec::new();
    for x in [0, 1] {
        for y in [2, 3] {
            for z in [4, 5] {
                faces.push(vec![x, y, z]);
            }
        }
    }
    Mesh::flat(verts, faces)
}

/// Icosahedron with edge length 2; faces found as triples of mutually adjacent vertices.
fn icosahedron_raw() -> (Vec<Vec3>, Vec<Vec<usize>>) {
    let p = (1.0 + 5f32.sqrt()) / 2.0;
    let mut verts = Vec::new();
    for a in [-1.0, 1.0] {
        for b in [-p, p] {
            verts.push(Vec3::new(0.0, a, b));
            verts.push(Vec3::new(a, b, 0.0));
            verts.push(Vec3::new(b, 0.0, a));
        }
    }
    let adjacent = |i: usize, j: usize| ((verts[i] - verts[j]).length() - 2.0).abs() < 1e-3;
    let n = verts.len();
    let mut faces = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            for k in j + 1..n {
                if adjacent(i, j) && adjacent(j, k) && adjacent(i, k) {
                    faces.push(vec![i, j, k]);
                }
            }
        }
    }
    (verts, faces)
}

/// Dual of the icosahedron: one vertex per icosahedron face, one pentagon per icosahedron vertex.
fn dodecahedron() -> Mesh {
    let (ico_verts, ico_faces) = icosahedron_raw();
    let verts: Vec<Vec3> = ico_faces
        .iter()
        .map(|f| f.iter().fold(Vec3::ZERO, |acc, &i| acc + ico_verts[i]) / 3.0)
        .collect();
    let faces = ico_verts
        .iter()
        .enumerate()
        .map(|(vi, &v)| {
            let axis = v.normalize();
            let mut ring: Vec<usize> = (0..ico_faces.len()).filter(|&fi| ico_faces[fi].contains(&vi)).collect();
            let first = verts[ring[0]];
            let u = (first - axis * first.dot(axis)).normalize();
            let w = axis.cross(u);
            let angle = |fi: usize| verts[fi].dot(w).atan2(verts[fi].dot(u));
            ring.sort_by(|&a, &b| angle(a).total_cmp(&angle(b)));
            ring
        })
        .collect();
    Mesh::flat(verts, faces)
}

fn sphere(lat: usize, lon: usize) -> Mesh {
    let mut verts = vec![Vec3::new(0.0, 1.0, 0.0)];
    for i in 1..lat {
        let theta = PI * i as f32 / lat as f32;
        for j in 0..lon {
            let phi = TAU * j as f32 / lon as f32;
            verts.push(Vec3::new(theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin()));
        }
    }
    verts.push(Vec3::new(0.0, -1.0, 0.0));
    let south = verts.len() - 1;
    let ring = |i: usize, j: usize| 1 + (i - 1) * lon + j % lon;

    let mut faces = Vec::new();
    for j in 0..lon {
        faces.push(vec![0, ring(1, j), ring(1, j + 1)]);
        for i in 1..lat - 1 {
            faces.push(vec![ring(i, j), ring(i + 1, j), ring(i + 1, j + 1), ring(i, j + 1)]);
        }
        faces.push(vec![south, ring(lat - 1, j + 1), ring(lat - 1, j)]);
    }
    let normals = verts.clone();
    Mesh::smooth(verts, normals, faces)
}

/// Torus lying in the XZ plane; `tube` is the tube radius relative to the ring radius 1.
fn torus(segments: usize, sides: usize, tube: f32) -> Mesh {
    let scale = 1.0 / (1.0 + tube);
    let mut verts = Vec::with_capacity(segments * sides);
    let mut normals = Vec::with_capacity(segments * sides);
    for i in 0..segments {
        let u = TAU * i as f32 / segments as f32;
        for j in 0..sides {
            let v = TAU * j as f32 / sides as f32;
            let r = 1.0 + tube * v.cos();
            verts.push(Vec3::new(r * u.cos(), tube * v.sin(), r * u.sin()) * scale);
            normals.push(Vec3::new(v.cos() * u.cos(), v.sin(), v.cos() * u.sin()));
        }
    }
    let idx = |i: usize, j: usize| (i % segments) * sides + j % sides;
    let mut faces = Vec::new();
    for i in 0..segments {
        for j in 0..sides {
            faces.push(vec![idx(i, j), idx(i + 1, j), idx(i + 1, j + 1), idx(i, j + 1)]);
        }
    }
    Mesh::smooth(verts, normals, faces)
}

/// 4D hypercube, rotated in 4D and perspective-projected down to 3D every frame.
pub struct Tesseract {
    verts: Vec<[f32; 4]>,
    pub edges: Vec<(usize, usize)>,
}

impl Tesseract {
    fn new() -> Self {
        let verts = (0..16)
            .map(|i| std::array::from_fn(|bit| if i >> bit & 1 == 1 { 0.5 } else { -0.5 }))
            .collect();
        let edges = (0..16usize)
            .flat_map(|i| (i + 1..16).map(move |j| (i, j)))
            .filter(|(i, j)| (i ^ j).count_ones() == 1)
            .collect();
        Self { verts, edges }
    }

    /// Rotates in the XW plane by `xw` and the ZW plane by `zw`, then projects along W.
    pub fn project(&self, xw: f32, zw: f32) -> Vec<Vec3> {
        let (s1, c1) = xw.sin_cos();
        let (s2, c2) = zw.sin_cos();
        self.verts
            .iter()
            .map(|&[x, y, z, w]| {
                let (x, w) = (x * c1 - w * s1, x * s1 + w * c1);
                let (z, w) = (z * c2 - w * s2, z * s2 + w * c2);
                let k = 1.6 / (2.0 - w);
                Vec3::new(x, y, z) * k
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(kind: ShapeKind) -> Mesh {
        match kind.build() {
            Model::Mesh(m) => m,
            Model::Tesseract(_) => panic!("not a mesh"),
        }
    }

    /// V - E + F, counting faces as polygons (triangles merged back by shared normal for flat meshes).
    fn euler(m: &Mesh, faces: usize) -> i64 {
        m.verts.len() as i64 - m.edges.len() as i64 + faces as i64
    }

    #[test]
    fn platonic_solids_have_expected_counts() {
        for (kind, v, e, f) in [
            (ShapeKind::Cube, 8, 12, 6),
            (ShapeKind::Octahedron, 6, 12, 8),
            (ShapeKind::Dodecahedron, 20, 30, 12),
            (ShapeKind::Icosahedron, 12, 30, 20),
            (ShapeKind::Pyramid, 5, 8, 5),
        ] {
            let m = mesh(kind);
            assert_eq!((m.verts.len(), m.edges.len(), m.normals.len()), (v, e, f), "{kind:?}");
            assert_eq!(euler(&m, f), 2, "{kind:?}");
        }
    }

    #[test]
    fn meshes_fit_unit_sphere() {
        for &kind in ShapeKind::value_variants() {
            if let Model::Mesh(m) = kind.build() {
                let r = m.verts.iter().map(|v| v.length()).fold(0.0, f32::max);
                assert!((r - 1.0).abs() < 1e-4, "{kind:?} radius {r}");
            }
        }
    }

    #[test]
    fn flat_normals_point_outward() {
        for kind in [ShapeKind::Cube, ShapeKind::Pyramid, ShapeKind::Octahedron, ShapeKind::Dodecahedron, ShapeKind::Icosahedron] {
            let m = mesh(kind);
            for tri in &m.tris {
                let centroid = (m.verts[tri[0].0] + m.verts[tri[1].0] + m.verts[tri[2].0]) / 3.0;
                assert!(m.normals[tri[0].1].dot(centroid) > 0.0, "{kind:?}");
            }
        }
    }

    #[test]
    fn dodecahedron_faces_are_planar_pentagons() {
        let m = mesh(ShapeKind::Dodecahedron);
        // 12 pentagons fan-triangulated into 3 triangles each.
        assert_eq!(m.tris.len(), 36);
        let edge_len = |(a, b): (usize, usize)| (m.verts[a] - m.verts[b]).length();
        let first = edge_len(m.edges[0]);
        assert!(m.edges.iter().all(|&e| (edge_len(e) - first).abs() < 1e-4));
    }

    #[test]
    fn torus_is_closed_surface_of_genus_one() {
        let m = mesh(ShapeKind::Torus);
        assert_eq!(euler(&m, 28 * 14), 0);
    }

    #[test]
    fn tesseract_has_32_edges_and_fits() {
        let t = Tesseract::new();
        assert_eq!(t.edges.len(), 32);
        for step in 0..50 {
            let a = step as f32 * 0.37;
            let max = t.project(a, a * 0.6).iter().map(|v| v.length()).fold(0.0, f32::max);
            assert!(max <= 1.0, "projected radius {max}");
        }
    }
}
