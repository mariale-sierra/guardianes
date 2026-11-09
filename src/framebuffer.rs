use raylib::prelude::Vector3;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub colors: Vec<Vector3>,
    pub depths: Vec<f32>,
    pub background: Vector3,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            colors: vec![Vector3::zero(); width * height],
            depths: vec![f32::INFINITY; width * height],
            background: Vector3::new(0.005, 0.005, 0.02),
        }
    }

    /// Reinicia colores Y profundidades. Se llama UNA vez por cuadro.
    pub fn clear(&mut self) {
        self.colors.fill(self.background);
        self.depths.fill(f32::INFINITY);
    }

    /// RGBA8 para actualizar la textura de raylib.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.width * self.height * 4);
        for c in &self.colors {
            out.push((c.x.clamp(0.0, 1.0) * 255.0) as u8);
            out.push((c.y.clamp(0.0, 1.0) * 255.0) as u8);
            out.push((c.z.clamp(0.0, 1.0) * 255.0) as u8);
            out.push(255);
        }
        out
    }
}
