use crate::*;

const SPORE_BOUND: f64 = 3000.0;

#[derive(Debug, Clone)]
pub struct Spore {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

impl Spore {
    pub fn random() -> Self {
        let x = (rand::random::<f64>() * 2.0 - 1.0) * SPORE_BOUND;
        let y = (rand::random::<f64>() * 2.0 - 1.0) * SPORE_BOUND;
        let radius = (rand::random::<f64>() * 3.0 + 10.0).max(5.0);
        Self {
            id: nanoid!(),
            x,
            y,
            radius,
        }
    }
}
