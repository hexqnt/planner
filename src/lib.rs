#![feature(portable_simd)]

mod app;
mod calendar;
mod ical;
mod model;
mod text;
mod vacation;

pub use app::Planner;

#[cfg(feature = "testing")]
pub use app::testing;

#[cfg(test)]
mod test_support;
