//! Loads the TLE catalog and sorts objects into categories.

use bevy::prelude::*;
use sgp4_rust::{Satellite, Tle, tle};

pub const CATALOG_PATH: &str = "assets/catalog.tle";

/// What kind of object a catalog entry is, judged from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Starlink,
    Payload,
    RocketBody,
    Debris,
}

impl Category {
    pub const ALL: [Category; 4] = [
        Category::Payload,
        Category::Starlink,
        Category::RocketBody,
        Category::Debris,
    ];

    pub fn from_name(name: &str) -> Category {
        if name.contains(" DEB") {
            Category::Debris
        } else if name.contains(" R/B") {
            Category::RocketBody
        } else if name.starts_with("STARLINK") {
            Category::Starlink
        } else {
            Category::Payload
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Category::Payload => "Active satellites",
            Category::Starlink => "Starlink",
            Category::RocketBody => "Rocket bodies",
            Category::Debris => "Debris",
        }
    }

    /// HDR color: values above 1 bloom.
    pub fn color(self) -> LinearRgba {
        match self {
            Category::Payload => LinearRgba::rgb(0.25, 1.9, 1.1),
            Category::Starlink => LinearRgba::rgb(0.55, 0.85, 2.1),
            Category::RocketBody => LinearRgba::rgb(2.1, 1.5, 0.3),
            Category::Debris => LinearRgba::rgb(2.4, 0.5, 0.18),
        }
    }
}

/// Breakups that produced large, well-known debris clouds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Breakup {
    Fengyun1C,
    Cosmos2251,
    Iridium33,
    Cosmos1408,
}

impl Breakup {
    pub const ALL: [Breakup; 4] = [
        Breakup::Fengyun1C,
        Breakup::Cosmos2251,
        Breakup::Iridium33,
        Breakup::Cosmos1408,
    ];

    fn name_prefix(self) -> &'static str {
        match self {
            Breakup::Fengyun1C => "FENGYUN 1C DEB",
            Breakup::Cosmos2251 => "COSMOS 2251 DEB",
            Breakup::Iridium33 => "IRIDIUM 33 DEB",
            Breakup::Cosmos1408 => "COSMOS 1408 DEB",
        }
    }

    pub fn from_name(name: &str) -> Option<Breakup> {
        Breakup::ALL
            .into_iter()
            .find(|b| name.starts_with(b.name_prefix()))
    }

    pub fn description(self) -> &'static str {
        match self {
            Breakup::Fengyun1C => "Fengyun-1C - Chinese anti-satellite test, Jan 2007",
            Breakup::Cosmos2251 => "Cosmos 2251 - collided with Iridium 33, Feb 2009",
            Breakup::Iridium33 => "Iridium 33 - collided with Cosmos 2251, Feb 2009",
            Breakup::Cosmos1408 => "Cosmos 1408 - Russian anti-satellite test, Nov 2021",
        }
    }
}

/// One tracked object: its TLE (for display) and its initialized propagator.
pub struct CatalogEntry {
    pub tle: Tle,
    pub satellite: Satellite,
    pub category: Category,
    pub breakup: Option<Breakup>,
}

impl CatalogEntry {
    pub fn name(&self) -> &str {
        self.tle.name.as_deref().unwrap_or("UNKNOWN")
    }
}

#[derive(Resource)]
pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
    /// Entries skipped because the TLE did not parse or SGP4 rejected it.
    pub rejected: usize,
}

impl Catalog {
    pub fn load(path: &str) -> Catalog {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {path}: {e}. Run scripts/fetch_catalog.sh"));
        let mut rejected = 0;
        let entries = tle::parse_all(&text)
            .filter_map(|parsed| {
                let entry = parsed.ok().and_then(|tle| {
                    let satellite = Satellite::from_tle(&tle).ok()?;
                    let name = tle.name.as_deref().unwrap_or("");
                    Some(CatalogEntry {
                        category: Category::from_name(name),
                        breakup: Breakup::from_name(name),
                        satellite,
                        tle,
                    })
                });
                if entry.is_none() {
                    rejected += 1;
                }
                entry
            })
            .collect();
        Catalog { entries, rejected }
    }

    pub fn count(&self, category: Category) -> usize {
        self.entries
            .iter()
            .filter(|e| e.category == category)
            .count()
    }
}
