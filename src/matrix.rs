// Matrices 4x4 propias (fila-mayor, vectores columna). No usamos las de raylib.
use raylib::prelude::Vector3;
use std::ops::Mul;

#[derive(Clone, Copy, Debug)]
pub struct Mat4(pub [[f32; 4]; 4]);

impl Mat4 {
    pub fn identity() -> Mat4 {
        Mat4([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    /// Multiplica por (x, y, z, w) y devuelve [x, y, z, w].
    pub fn mul_vec4(&self, v: Vector3, w: f32) -> [f32; 4] {
        let m = &self.0;
        let mut r = [0.0; 4];
        for i in 0..4 {
            r[i] = m[i][0] * v.x + m[i][1] * v.y + m[i][2] * v.z + m[i][3] * w;
        }
        r
    }

    pub fn transform_point(&self, v: Vector3) -> Vector3 {
        let r = self.mul_vec4(v, 1.0);
        Vector3::new(r[0], r[1], r[2])
    }

    /// Solo la parte lineal (sirve para normales con escala uniforme).
    pub fn transform_dir(&self, v: Vector3) -> Vector3 {
        let r = self.mul_vec4(v, 0.0);
        Vector3::new(r[0], r[1], r[2])
    }
}

impl Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, o: Mat4) -> Mat4 {
        let mut r = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    r[i][j] += self.0[i][k] * o.0[k][j];
                }
            }
        }
        Mat4(r)
    }
}

fn rotation_x(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    Mat4([[1.0, 0.0, 0.0, 0.0], [0.0, c, -s, 0.0], [0.0, s, c, 0.0], [0.0, 0.0, 0.0, 1.0]])
}
fn rotation_y(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    Mat4([[c, 0.0, s, 0.0], [0.0, 1.0, 0.0, 0.0], [-s, 0.0, c, 0.0], [0.0, 0.0, 0.0, 1.0]])
}
fn rotation_z(a: f32) -> Mat4 {
    let (s, c) = a.sin_cos();
    Mat4([[c, -s, 0.0, 0.0], [s, c, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])
}

/// M = T * (Ry * Rx * Rz) * S : primero inclina en X, luego orienta en Y.
pub fn create_model_matrix(translation: Vector3, scale: f32, rotation: Vector3) -> Mat4 {
    let r = rotation_y(rotation.y) * rotation_x(rotation.x) * rotation_z(rotation.z);
    let mut m = r;
    for i in 0..3 {
        for j in 0..3 {
            m.0[i][j] *= scale;
        }
    }
    m.0[0][3] = translation.x;
    m.0[1][3] = translation.y;
    m.0[2][3] = translation.z;
    m
}

pub fn create_view_matrix(eye: Vector3, target: Vector3, up: Vector3) -> Mat4 {
    let f = (target - eye).normalized();
    let r = f.cross(up).normalized();
    let u = r.cross(f);
    Mat4([
        [r.x, r.y, r.z, -r.dot(eye)],
        [u.x, u.y, u.z, -u.dot(eye)],
        [-f.x, -f.y, -f.z, f.dot(eye)],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

pub fn create_projection_matrix(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y * 0.5).tan();
    Mat4([
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, (far + near) / (near - far), 2.0 * far * near / (near - far)],
        [0.0, 0.0, -1.0, 0.0],
    ])
}
