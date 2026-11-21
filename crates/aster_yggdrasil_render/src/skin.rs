use crate::{
    geometry::{BoxUv, Camera, Cuboid, DrawFace, Vec3, cuboid_faces, inflate, uv},
    options::SkinModel,
};

/// Texture metadata derived from the decoded skin image.
///
/// `ratio` maps vanilla 64x64 UV units to the actual decoded image size, so HD
/// skins can reuse the same UV constants. `legacy` marks 64x32 skins where left
/// limbs and all outer layers are absent.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TextureLayout {
    pub(crate) ratio: f32,
    pub(crate) legacy: bool,
}

#[derive(Debug, Clone, Copy)]
struct BodyLayout {
    arm_width: f32,
}

pub(crate) fn collect_player_faces(
    faces: &mut Vec<DrawFace>,
    camera: Camera,
    model: SkinModel,
    texture_layout: TextureLayout,
    show_outer_layer: bool,
) {
    // Minecraft player geometry is built from six boxes in model units. The
    // coordinates match vanilla proportions: head 8x8x8, body 8x12x4, legs
    // 4x12x4, and arms either 4x12x4 or 3x12x4 for slim/Alex skins.
    let layout = BodyLayout {
        arm_width: match model {
            SkinModel::Default => 4.0,
            SkinModel::Slim => 3.0,
        },
    };
    let arm = layout.arm_width;
    let left_arm_min_x = 4.0;
    let left_arm_max_x = 4.0 + arm;
    let right_arm_min_x = -4.0 - arm;
    let right_arm_max_x = -4.0;

    add_cuboid_faces(
        faces,
        camera,
        Cuboid {
            min: Vec3::new(-4.0, 12.0, -4.0),
            max: Vec3::new(4.0, 20.0, 4.0),
        },
        head_uv(false),
        false,
    );
    add_cuboid_faces(
        faces,
        camera,
        Cuboid {
            min: Vec3::new(-4.0, 0.0, -2.0),
            max: Vec3::new(4.0, 12.0, 2.0),
        },
        body_uv(false),
        false,
    );
    add_cuboid_faces(
        faces,
        camera,
        Cuboid {
            min: Vec3::new(right_arm_min_x, 0.0, -2.0),
            max: Vec3::new(right_arm_max_x, 12.0, 2.0),
        },
        right_arm_uv(false, arm),
        false,
    );
    add_cuboid_faces(
        faces,
        camera,
        Cuboid {
            min: Vec3::new(left_arm_min_x, 0.0, -2.0),
            max: Vec3::new(left_arm_max_x, 12.0, 2.0),
        },
        left_arm_uv(false, texture_layout.legacy, arm),
        texture_layout.legacy,
    );
    add_cuboid_faces(
        faces,
        camera,
        Cuboid {
            min: Vec3::new(-4.0, -12.0, -2.0),
            max: Vec3::new(0.0, 0.0, 2.0),
        },
        right_leg_uv(false),
        false,
    );
    add_cuboid_faces(
        faces,
        camera,
        Cuboid {
            min: Vec3::new(0.0, -12.0, -2.0),
            max: Vec3::new(4.0, 0.0, 2.0),
        },
        left_leg_uv(false, texture_layout.legacy),
        texture_layout.legacy,
    );

    if show_outer_layer && !texture_layout.legacy {
        add_outer_layers(
            faces,
            camera,
            right_arm_min_x,
            right_arm_max_x,
            left_arm_min_x,
            left_arm_max_x,
        );
    }
}

fn add_outer_layers(
    faces: &mut Vec<DrawFace>,
    camera: Camera,
    right_arm_min_x: f32,
    right_arm_max_x: f32,
    left_arm_min_x: f32,
    left_arm_max_x: f32,
) {
    // Second-layer cuboids sit slightly outside the base model. The head layer
    // uses Minecraft's larger hat offset; body and limb layers use smaller
    // offsets to avoid excessive overlap in static previews.
    add_cuboid_faces(
        faces,
        camera,
        inflate(
            Cuboid {
                min: Vec3::new(-4.0, 12.0, -4.0),
                max: Vec3::new(4.0, 20.0, 4.0),
            },
            0.5,
        ),
        head_uv(true),
        false,
    );
    add_cuboid_faces(
        faces,
        camera,
        inflate(
            Cuboid {
                min: Vec3::new(-4.0, 0.0, -2.0),
                max: Vec3::new(4.0, 12.0, 2.0),
            },
            0.28,
        ),
        body_uv(true),
        false,
    );
    add_cuboid_faces(
        faces,
        camera,
        inflate(
            Cuboid {
                min: Vec3::new(right_arm_min_x, 0.0, -2.0),
                max: Vec3::new(right_arm_max_x, 12.0, 2.0),
            },
            0.24,
        ),
        right_arm_uv(true, right_arm_max_x - right_arm_min_x),
        false,
    );
    add_cuboid_faces(
        faces,
        camera,
        inflate(
            Cuboid {
                min: Vec3::new(left_arm_min_x, 0.0, -2.0),
                max: Vec3::new(left_arm_max_x, 12.0, 2.0),
            },
            0.24,
        ),
        left_arm_uv(true, false, left_arm_max_x - left_arm_min_x),
        false,
    );
    add_cuboid_faces(
        faces,
        camera,
        inflate(
            Cuboid {
                min: Vec3::new(-4.0, -12.0, -2.0),
                max: Vec3::new(0.0, 0.0, 2.0),
            },
            0.24,
        ),
        right_leg_uv(true),
        false,
    );
    add_cuboid_faces(
        faces,
        camera,
        inflate(
            Cuboid {
                min: Vec3::new(0.0, -12.0, -2.0),
                max: Vec3::new(4.0, 0.0, 2.0),
            },
            0.24,
        ),
        left_leg_uv(true, false),
        false,
    );
}

fn add_cuboid_faces(
    faces: &mut Vec<DrawFace>,
    camera: Camera,
    cuboid: Cuboid,
    uv: BoxUv,
    mirror_u: bool,
) {
    for face in cuboid_faces(cuboid, uv, mirror_u) {
        let points = [
            camera.project(face.corners[0]),
            camera.project(face.corners[1]),
            camera.project(face.corners[2]),
            camera.project(face.corners[3]),
        ];
        let depth = points.iter().map(|point| point.depth).sum::<f32>() / 4.0;
        // We keep every face rather than culling by normal direction. Culling
        // opens small gaps at cuboid seams after projection, especially around
        // head and outer-layer edges. Depth sorting handles coverage later.
        faces.push(DrawFace {
            points,
            uv: face.uv,
            mirror_u: face.mirror_u,
            rotate_uv_180: face.rotate_uv_180,
            depth,
        });
    }
}

fn head_uv(layer: bool) -> BoxUv {
    let x = if layer { 32.0 } else { 0.0 };
    BoxUv {
        top: uv(x + 8.0, 0.0, 8.0, 8.0),
        bottom: uv(x + 16.0, 0.0, 8.0, 8.0),
        right: uv(x, 8.0, 8.0, 8.0),
        front: uv(x + 8.0, 8.0, 8.0, 8.0),
        left: uv(x + 16.0, 8.0, 8.0, 8.0),
        back: uv(x + 24.0, 8.0, 8.0, 8.0),
    }
}

fn body_uv(layer: bool) -> BoxUv {
    let y = if layer { 32.0 } else { 16.0 };
    BoxUv {
        top: uv(20.0, y, 8.0, 4.0),
        bottom: uv(28.0, y, 8.0, 4.0),
        right: uv(16.0, y + 4.0, 4.0, 12.0),
        front: uv(20.0, y + 4.0, 8.0, 12.0),
        left: uv(28.0, y + 4.0, 4.0, 12.0),
        back: uv(32.0, y + 4.0, 8.0, 12.0),
    }
}

fn right_arm_uv(layer: bool, arm_width: f32) -> BoxUv {
    let y = if layer { 32.0 } else { 16.0 };
    // Slim arms consume 3 pixels of horizontal UV for front/back/top/bottom,
    // but the side depth remains 4 pixels. Steve arms use 4 pixels throughout.
    let x1 = 44.0 + arm_width;
    BoxUv {
        top: uv(44.0, y, arm_width, 4.0),
        bottom: uv(x1, y, arm_width, 4.0),
        right: uv(40.0, y + 4.0, 4.0, 12.0),
        front: uv(44.0, y + 4.0, arm_width, 12.0),
        left: uv(x1, y + 4.0, 4.0, 12.0),
        back: uv(x1 + 4.0, y + 4.0, arm_width, 12.0),
    }
}

fn left_arm_uv(layer: bool, legacy: bool, arm_width: f32) -> BoxUv {
    if legacy {
        // 64x32 skins have no separate left arm. Reuse the right arm and mirror
        // it at face construction time.
        return right_arm_uv(layer, arm_width);
    }
    let x = if layer { 48.0 } else { 32.0 };
    let x1 = x + 4.0 + arm_width;
    BoxUv {
        top: uv(x + 4.0, 48.0, arm_width, 4.0),
        bottom: uv(x1, 48.0, arm_width, 4.0),
        right: uv(x, 52.0, 4.0, 12.0),
        front: uv(x + 4.0, 52.0, arm_width, 12.0),
        left: uv(x1, 52.0, 4.0, 12.0),
        back: uv(x1 + 4.0, 52.0, arm_width, 12.0),
    }
}

fn right_leg_uv(layer: bool) -> BoxUv {
    let y = if layer { 32.0 } else { 16.0 };
    BoxUv {
        top: uv(4.0, y, 4.0, 4.0),
        bottom: uv(8.0, y, 4.0, 4.0),
        right: uv(0.0, y + 4.0, 4.0, 12.0),
        front: uv(4.0, y + 4.0, 4.0, 12.0),
        left: uv(8.0, y + 4.0, 4.0, 12.0),
        back: uv(12.0, y + 4.0, 4.0, 12.0),
    }
}

fn left_leg_uv(layer: bool, legacy: bool) -> BoxUv {
    if legacy {
        return right_leg_uv(layer);
    }
    let x = if layer { 0.0 } else { 16.0 };
    BoxUv {
        top: uv(x + 4.0, 48.0, 4.0, 4.0),
        bottom: uv(x + 8.0, 48.0, 4.0, 4.0),
        right: uv(x, 52.0, 4.0, 12.0),
        front: uv(x + 4.0, 52.0, 4.0, 12.0),
        left: uv(x + 8.0, 52.0, 4.0, 12.0),
        back: uv(x + 12.0, 52.0, 4.0, 12.0),
    }
}
