use crate::math::*;

#[derive(Debug, Clone, Copy)]
pub struct Box3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl Box3 {
    pub fn from_3points(p1: &Vec3, p2: &Vec3, p3: &Vec3) -> Box3 {
        Box3 {
            min: p1.min(p2).min(p3),
            max: p1.max(p2).max(p3),
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    pub fn extend_by_box(&mut self, the_other_box: &Box3) {
        self.min.min(&the_other_box.min);
        self.max.max(&the_other_box.max);
    }
}
