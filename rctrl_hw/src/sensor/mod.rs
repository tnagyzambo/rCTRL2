mod kellerpa7lc;
pub use kellerpa7lc::KellerPA7LC;

pub trait Sensor {
    type Output;

    fn conversion(&self, voltage: f64) -> Self::Output;
}
