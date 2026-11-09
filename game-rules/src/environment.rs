use bevy::prelude::IVec2;
use rand::{Rng, RngExt};

pub const RAIN_INTERVAL: std::ops::RangeInclusive<u16> = 10..=20;
pub const EARTHQUAKE_INTERVAL: std::ops::RangeInclusive<u16> = 20..=30;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindDirection {
    #[default]
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl WindDirection {
    pub fn from_label(label: &str) -> Option<Self> {
        Some(match label {
            "N" => Self::North,
            "NE" => Self::NorthEast,
            "E" => Self::East,
            "SE" => Self::SouthEast,
            "S" => Self::South,
            "SW" => Self::SouthWest,
            "W" => Self::West,
            "NW" => Self::NorthWest,
            _ => return None,
        })
    }

    pub fn random(rng: &mut impl Rng) -> Self {
        const DIRECTIONS: [WindDirection; 8] = [
            WindDirection::North,
            WindDirection::NorthEast,
            WindDirection::East,
            WindDirection::SouthEast,
            WindDirection::South,
            WindDirection::SouthWest,
            WindDirection::West,
            WindDirection::NorthWest,
        ];
        DIRECTIONS[rng.random_range(0..DIRECTIONS.len())]
    }

    pub fn offset(self) -> IVec2 {
        match self {
            Self::North => IVec2::new(0, -1),
            Self::NorthEast => IVec2::new(1, -1),
            Self::East => IVec2::new(1, 0),
            Self::SouthEast => IVec2::new(1, 1),
            Self::South => IVec2::new(0, 1),
            Self::SouthWest => IVec2::new(-1, 1),
            Self::West => IVec2::new(-1, 0),
            Self::NorthWest => IVec2::new(-1, -1),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::North => "N",
            Self::NorthEast => "NE",
            Self::East => "E",
            Self::SouthEast => "SE",
            Self::South => "S",
            Self::SouthWest => "SW",
            Self::West => "W",
            Self::NorthWest => "NW",
        }
    }
}
