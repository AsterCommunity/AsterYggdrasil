pub(crate) const EPSILON: f32 = 0.0001;

/// Minimal 3D vector used by the software projector.
///
/// Coordinates follow Minecraft player proportions: Y is vertical, X is
/// left/right, and Z is front/back before camera rotation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Vec3 {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
}

impl Vec3 {
    pub(crate) const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Vec2 {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Camera {
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) scale: f32,
    pub(crate) center_x: f32,
    pub(crate) center_y: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectedPoint {
    pub(crate) position: Vec2,
    pub(crate) depth: f32,
}

impl Camera {
    /// Rotate a model-space point by yaw and pitch, then project it
    /// orthographically into output-space coordinates.
    ///
    /// The returned depth is the rotated Z value. It is used for painter-style
    /// face ordering, which is sufficient for these small Minecraft cuboids.
    pub(crate) fn project(self, point: Vec3) -> ProjectedPoint {
        let yaw_sin = self.yaw.sin();
        let yaw_cos = self.yaw.cos();
        let pitch_sin = self.pitch.sin();
        let pitch_cos = self.pitch.cos();

        let x = yaw_cos.mul_add(point.x, yaw_sin * point.z);
        let z = (-yaw_sin).mul_add(point.x, yaw_cos * point.z);
        let y = pitch_cos.mul_add(point.y, -pitch_sin * z);
        let depth = pitch_sin.mul_add(point.y, pitch_cos * z);

        ProjectedPoint {
            position: Vec2 {
                x: self.center_x + x * self.scale,
                y: self.center_y - y * self.scale,
            },
            depth,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Cuboid {
    pub(crate) min: Vec3,
    pub(crate) max: Vec3,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Face {
    pub(crate) corners: [Vec3; 4],
    pub(crate) uv: UvRect,
    /// Mirror the sampled U axis. Legacy 64x32 skins reuse right limbs for the
    /// missing left limbs, so their UVs need this horizontal flip.
    pub(crate) mirror_u: bool,
    /// Minecraft bottom faces are laid out opposite to the default quad corner
    /// order used by the rasterizer, so they need a 180-degree UV rotation.
    pub(crate) rotate_uv_180: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct UvRect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct DrawFace {
    pub(crate) points: [ProjectedPoint; 4],
    pub(crate) uv: UvRect,
    pub(crate) mirror_u: bool,
    pub(crate) rotate_uv_180: bool,
    pub(crate) depth: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BoxUv {
    pub(crate) front: UvRect,
    pub(crate) back: UvRect,
    pub(crate) left: UvRect,
    pub(crate) right: UvRect,
    pub(crate) top: UvRect,
    pub(crate) bottom: UvRect,
}

pub(crate) fn uv(x: f32, y: f32, width: f32, height: f32) -> UvRect {
    UvRect {
        x,
        y,
        width,
        height,
    }
}

pub(crate) fn inflate(cuboid: Cuboid, amount: f32) -> Cuboid {
    Cuboid {
        min: Vec3::new(
            cuboid.min.x - amount,
            cuboid.min.y - amount,
            cuboid.min.z - amount,
        ),
        max: Vec3::new(
            cuboid.max.x + amount,
            cuboid.max.y + amount,
            cuboid.max.z + amount,
        ),
    }
}

/// Expand a cuboid into six textured faces.
///
/// The corner order is chosen to make each quad stable when later split into
/// two triangles. We keep all six faces; visibility decisions happen by depth
/// ordering in the engine instead of by backface culling.
pub(crate) fn cuboid_faces(cuboid: Cuboid, uv: BoxUv, mirror_u: bool) -> [Face; 6] {
    let x0 = cuboid.min.x;
    let x1 = cuboid.max.x;
    let y0 = cuboid.min.y;
    let y1 = cuboid.max.y;
    let z0 = cuboid.min.z;
    let z1 = cuboid.max.z;

    [
        Face {
            corners: [
                Vec3::new(x0, y0, z1),
                Vec3::new(x1, y0, z1),
                Vec3::new(x1, y1, z1),
                Vec3::new(x0, y1, z1),
            ],
            uv: uv.front,
            mirror_u,
            rotate_uv_180: false,
        },
        Face {
            corners: [
                Vec3::new(x1, y0, z0),
                Vec3::new(x0, y0, z0),
                Vec3::new(x0, y1, z0),
                Vec3::new(x1, y1, z0),
            ],
            uv: uv.back,
            mirror_u,
            rotate_uv_180: false,
        },
        Face {
            corners: [
                Vec3::new(x1, y0, z1),
                Vec3::new(x1, y0, z0),
                Vec3::new(x1, y1, z0),
                Vec3::new(x1, y1, z1),
            ],
            uv: uv.left,
            mirror_u,
            rotate_uv_180: false,
        },
        Face {
            corners: [
                Vec3::new(x0, y0, z0),
                Vec3::new(x0, y0, z1),
                Vec3::new(x0, y1, z1),
                Vec3::new(x0, y1, z0),
            ],
            uv: uv.right,
            mirror_u,
            rotate_uv_180: false,
        },
        Face {
            corners: [
                Vec3::new(x0, y1, z1),
                Vec3::new(x1, y1, z1),
                Vec3::new(x1, y1, z0),
                Vec3::new(x0, y1, z0),
            ],
            uv: uv.top,
            mirror_u,
            rotate_uv_180: false,
        },
        Face {
            corners: [
                Vec3::new(x0, y0, z0),
                Vec3::new(x1, y0, z0),
                Vec3::new(x1, y0, z1),
                Vec3::new(x0, y0, z1),
            ],
            uv: uv.bottom,
            mirror_u,
            rotate_uv_180: true,
        },
    ]
}
