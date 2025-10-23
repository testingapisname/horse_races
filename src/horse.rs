use rand::Rng;

pub struct Horse {
    pub name: String,
    pub position: f32,
    pub speed: f32,
    pub momentum: f32,
}

impl Horse {
    pub fn new(name: &str) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            name: name.to_string(),
            position: 0.0,
            speed: rng.gen_range(0.3..1.0),
            momentum: 0.0,
        }
    }

    pub fn advance(&mut self) {
        let mut rng = rand::thread_rng();
        // smooth acceleration/deceleration
        let accel: f32 = rng.gen_range(-0.2..0.4);
        self.momentum = (self.momentum + accel).clamp(0.0, self.speed * 2.0);
        self.position += self.momentum;
    }
}
