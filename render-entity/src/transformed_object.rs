use rendiation_math_entity::Transformation;

pub trait TransformedObject{
    fn get_transform(&self) -> Transformation;
}