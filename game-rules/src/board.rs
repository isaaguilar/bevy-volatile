use std::collections::VecDeque;

use bevy::prelude::*;
use rand::{Rng, seq::SliceRandom};

use crate::aquatic::{AquaticCreature, AquaticImpact};
use crate::thorn::{ACTIVE_TURNS, ThornGarden};
use crate::void::VoidGrowth;

pub const COLUMNS: usize = 10;
pub const ROWS: usize = 7;
pub const MAX_UNSAFE_TILES: usize = COLUMNS * ROWS * 60 / 100;
pub const PLAYER_STARTS: [IVec2; 2] = [IVec2::new(2, 3), IVec2::new(7, 3)];
const RIDGE_COUNT: usize = 6;
const WATER_COUNT: usize = 10;
#[cfg(any(test, feature = "test-support"))]
const WATER_TURNS: u8 = 5;
const POISON_TURNS: u8 = 2;
const LIFE_ESSENCE_TURNS: u8 = 3;
const ACID_TURNS: u8 = 5;
const ELECTRIFIED_ACID_TURNS: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ground {
    Solid,
    Water,
    Ridge,
    Hole,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TerrainGrowthKind {
    Void,
    Stone,
}

#[derive(Clone)]
pub struct VisualTile {
    pub ground: Ground,
    pub wet: u8,
    pub oil: bool,
    pub frozen: bool,
    pub poison: bool,
    pub acid: bool,
    pub electrified: bool,
    pub life: bool,
    pub thorns: bool,
    pub growth: Option<TerrainGrowthKind>,
}

impl Ground {
    pub fn walkable(self) -> bool {
        matches!(self, Self::Solid | Self::Water)
    }

    pub fn color(self) -> Color {
        match self {
            Self::Solid => Color::srgb(0.88, 0.83, 0.71),
            Self::Water => Color::srgb(0.30, 0.64, 0.78),
            Self::Ridge => Color::srgb(0.42, 0.28, 0.19),
            Self::Hole => Color::srgb(0.10, 0.08, 0.12),
        }
    }
}

#[derive(Resource, Clone)]
pub struct Board {
    tiles: [Ground; COLUMNS * ROWS],
    wet: [u8; COLUMNS * ROWS],
    oil: [bool; COLUMNS * ROWS],
    expired_oil_sources: u32,
    life_essence: [u8; COLUMNS * ROWS],
    life_sources: [u32; COLUMNS * ROWS],
    expired_life_sources: u32,
    frozen: [bool; COLUMNS * ROWS],
    poison: Vec<PoisonWave>,
    acid: [Option<Acid>; COLUMNS * ROWS],
    thorns: ThornGarden,
    aquatic_creatures: Vec<AquaticCreature>,
    next_aquatic_id: u64,
    terrain_growth: Vec<(TerrainGrowthKind, VoidGrowth)>,
    new_holes: Vec<IVec2>,
    new_unsafe: Vec<IVec2>,
    terrain_limit_hit: bool,
    visual_thorns: Option<[bool; COLUMNS * ROWS]>,
    visual_growth: Option<[Option<TerrainGrowthKind>; COLUMNS * ROWS]>,
}

#[derive(Clone, Copy)]
struct Acid {
    permanent: bool,
    turns: u8,
    fresh: bool,
    electric_turns: u8,
    electric_fresh: bool,
}

#[derive(Clone)]
struct PoisonWave {
    remaining: [u8; COLUMNS * ROWS],
    visited: [bool; COLUMNS * ROWS],
    fresh: bool,
}

impl Default for Board {
    fn default() -> Self {
        Self {
            tiles: [Ground::Solid; COLUMNS * ROWS],
            wet: [0; COLUMNS * ROWS],
            oil: [false; COLUMNS * ROWS],
            expired_oil_sources: 0,
            life_essence: [0; COLUMNS * ROWS],
            life_sources: [0; COLUMNS * ROWS],
            expired_life_sources: 0,
            frozen: [false; COLUMNS * ROWS],
            poison: Vec::new(),
            acid: [None; COLUMNS * ROWS],
            thorns: ThornGarden::default(),
            aquatic_creatures: Vec::new(),
            next_aquatic_id: 0,
            terrain_growth: Vec::new(),
            new_holes: Vec::new(),
            new_unsafe: Vec::new(),
            terrain_limit_hit: false,
            visual_thorns: None,
            visual_growth: None,
        }
    }
}

impl Board {
    pub fn from_visual(
        tiles: &[VisualTile],
        aquatic: &[crate::aquatic::AquaticVisual],
    ) -> Result<Self, &'static str> {
        if tiles.len() != COLUMNS * ROWS {
            return Err("Invalid board size");
        }
        if aquatic.iter().any(|visual| {
            Self::index(visual.origin).is_none() || visual.lengths.iter().any(|length| *length > 3)
        }) {
            return Err("Invalid aquatic creature");
        }
        let mut board = Self::default();
        let mut thorns = [false; COLUMNS * ROWS];
        let mut growth = [None; COLUMNS * ROWS];
        for (index, tile) in tiles.iter().enumerate() {
            if tile.wet > 100 || tile.wet % 20 != 0 || (tile.electrified && !tile.acid) {
                return Err("Invalid tile state");
            }
            board.tiles[index] = tile.ground;
            board.wet[index] = tile.wet / 20;
            board.oil[index] = tile.oil;
            board.frozen[index] = tile.frozen;
            board.life_essence[index] = u8::from(tile.life);
            if tile.acid {
                board.acid[index] = Some(Acid {
                    permanent: true,
                    turns: 0,
                    fresh: false,
                    electric_turns: u8::from(tile.electrified),
                    electric_fresh: false,
                });
            }
            if tile.poison {
                let cell = IVec2::new((index % COLUMNS) as i32, (index / COLUMNS) as i32);
                board.add_poison(cell);
            }
            thorns[index] = tile.thorns;
            growth[index] = tile.growth;
        }
        board.visual_thorns = Some(thorns);
        board.visual_growth = Some(growth);
        board.aquatic_creatures = aquatic.iter().map(AquaticCreature::from_visual).collect();
        Ok(board)
    }

    pub fn generate(rng: &mut impl Rng) -> Self {
        let mut board = Self::default();
        let mut candidates: Vec<_> = (0..ROWS)
            .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
            .filter(|cell| {
                PLAYER_STARTS
                    .iter()
                    .all(|start| (*cell - *start).abs().max_element() > 1)
            })
            .collect();
        candidates.shuffle(rng);
        let mut ridges = 0;
        for cell in candidates {
            board.set(cell, Ground::Ridge);
            if board.walkable_connected() {
                ridges += 1;
                if ridges == RIDGE_COUNT {
                    break;
                }
            } else {
                board.set(cell, Ground::Solid);
            }
        }

        let mut water_candidates: Vec<_> = (0..ROWS)
            .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
            .filter(|cell| {
                board.ground(*cell) == Some(Ground::Solid) && !PLAYER_STARTS.contains(cell)
            })
            .collect();
        water_candidates.shuffle(rng);
        for cell in water_candidates.into_iter().take(WATER_COUNT) {
            board.set(cell, Ground::Water);
        }
        board
    }

    pub fn ground(&self, cell: IVec2) -> Option<Ground> {
        Self::index(cell).map(|index| self.tiles[index])
    }

    pub fn is_wet(&self, cell: IVec2) -> bool {
        self.water_percentage(cell) > 0
    }

    pub fn water_percentage(&self, cell: IVec2) -> u8 {
        self.wet[Self::index(cell).expect("board cell must be in bounds")] * 20
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn make_wet(&mut self, cell: IVec2) {
        self.wet[Self::index(cell).expect("board cell must be in bounds")] = WATER_TURNS;
    }

    pub fn make_water(&mut self, cell: IVec2) {
        let index = Self::index(cell).expect("board cell must be in bounds");
        self.tiles[index] = Ground::Water;
        self.wet[index] = 0;
        if let Some(acid) = &mut self.acid[index] {
            acid.permanent = true;
        }
    }

    pub fn dry(&mut self, cell: IVec2) {
        let index = Self::index(cell).expect("board cell must be in bounds");
        self.wet[index] = 0;
        if self.tiles[index] != Ground::Water {
            self.expire_life_essence(index);
        }
    }

    pub fn burn_water(&mut self, cell: IVec2) {
        assert!(self.is_water(cell) && !self.is_frozen(cell));
        self.dry(cell);
        self.set(cell, Ground::Solid);
        self.clear_acid(cell);
        self.clear_poison(cell);
        self.burn_thorns(&[cell]);
        for (_, growth) in &mut self.terrain_growth {
            growth.clear_cell(cell);
        }
    }

    pub fn has_oil(&self, cell: IVec2) -> bool {
        self.oil[Self::index(cell).expect("board cell must be in bounds")]
    }

    pub fn add_oil(&mut self, cell: IVec2) {
        self.oil[Self::index(cell).expect("board cell must be in bounds")] = true;
    }

    pub fn remove_oil(&mut self, cell: IVec2) {
        let oil = &mut self.oil[Self::index(cell).expect("board cell must be in bounds")];
        if std::mem::take(oil) {
            self.expired_oil_sources += 1;
        }
    }

    pub fn take_expired_oil_sources(&mut self) -> u32 {
        std::mem::take(&mut self.expired_oil_sources)
    }

    pub fn is_water(&self, cell: IVec2) -> bool {
        match self.ground(cell) {
            Some(Ground::Water) => true,
            Some(Ground::Solid) => self.is_wet(cell),
            _ => false,
        }
    }

    pub fn has_life_essence(&self, cell: IVec2) -> bool {
        Self::index(cell).is_some_and(|index| self.life_essence[index] > 0 && self.is_water(cell))
    }

    pub fn add_life_essence(&mut self, cell: IVec2) {
        assert!(self.is_water(cell), "life essence requires water");
        let index = Self::index(cell).expect("board cell must be in bounds");
        self.life_essence[index] = LIFE_ESSENCE_TURNS;
        self.life_sources[index] += 1;
    }

    fn expire_life_essence(&mut self, index: usize) {
        self.life_essence[index] = 0;
        self.expired_life_sources += std::mem::take(&mut self.life_sources[index]);
    }

    pub fn finish_life_turn(&mut self) {
        for index in 0..self.life_essence.len() {
            if self.life_essence[index] > 0 {
                self.life_essence[index] -= 1;
                if self.life_essence[index] == 0 {
                    self.expire_life_essence(index);
                }
            }
        }
    }

    pub fn take_expired_life_sources(&mut self) -> u32 {
        std::mem::take(&mut self.expired_life_sources)
    }

    pub fn activate_thornheart(&mut self, cell: IVec2, seed: u64, life_owed: bool) {
        let index = Self::index(cell).expect("board cell must be in bounds");
        let active_turns = if self.has_life_essence(cell) {
            ACTIVE_TURNS - LIFE_ESSENCE_TURNS + self.life_essence[index]
        } else {
            ACTIVE_TURNS
        };
        self.thorns.activate(cell, seed, life_owed, active_turns);
    }

    pub fn has_thorns(&self, cell: IVec2) -> bool {
        self.visual_thorns
            .as_ref()
            .and_then(|thorns| Self::index(cell).map(|index| thorns[index]))
            .unwrap_or_else(|| self.thorns.contains(cell))
    }

    pub fn activate_aquatic(&mut self, cell: IVec2) {
        assert!(self.is_water(cell), "aquatic creature requires water");
        self.aquatic_creatures
            .push(AquaticCreature::new(cell, self.next_aquatic_id));
        self.next_aquatic_id += 1;
    }

    pub fn aquatic_cells(&self) -> impl Iterator<Item = IVec2> + '_ {
        self.aquatic_creatures
            .iter()
            .flat_map(AquaticCreature::cells)
    }

    pub fn aquatic_visuals(&self) -> impl Iterator<Item = crate::aquatic::AquaticVisual> + '_ {
        self.aquatic_creatures
            .iter()
            .map(|creature| creature.visual(None))
    }

    pub fn aquatic_contacts(&self, cell: IVec2) -> impl Iterator<Item = usize> + '_ {
        self.aquatic_creatures
            .iter()
            .enumerate()
            .filter_map(move |(index, creature)| {
                creature.cells().any(|part| part == cell).then_some(index)
            })
    }

    pub fn finish_aquatic_turn(&mut self) -> Vec<Option<AquaticImpact>> {
        let mut creatures = std::mem::take(&mut self.aquatic_creatures);
        let mut impacts = Vec::with_capacity(creatures.len());
        creatures.retain_mut(|creature| {
            let impact = creature.advance(self);
            let active = !creature.finished();
            impacts.push(impact);
            active
        });
        self.aquatic_creatures = creatures;
        impacts
    }

    pub fn burn_thorns(&mut self, cells: &[IVec2]) {
        self.thorns.burn(cells);
    }

    pub fn dissolve_thorns(&mut self, cells: &[IVec2]) {
        self.thorns.dissolve(cells);
    }

    pub fn is_acidic(&self, cell: IVec2) -> bool {
        Self::index(cell).is_some_and(|index| self.acid[index].is_some())
    }

    pub fn is_electrified_acid(&self, cell: IVec2) -> bool {
        Self::index(cell)
            .is_some_and(|index| self.acid[index].is_some_and(|acid| acid.electric_turns > 0))
    }

    pub fn clear_acid(&mut self, cell: IVec2) {
        let index = Self::index(cell).expect("acid cell must be in bounds");
        self.acid[index] = None;
    }

    pub fn clear_life_essence(&mut self, cell: IVec2) {
        let index = Self::index(cell).expect("life essence cell must be in bounds");
        self.expire_life_essence(index);
    }

    pub fn can_create_unsafe(&self, cells: &[IVec2]) -> bool {
        let current = self
            .tiles
            .iter()
            .filter(|ground| !ground.walkable())
            .count();
        let mut added = Vec::new();
        for &cell in cells {
            if self.ground(cell).is_some_and(Ground::walkable) && !added.contains(&cell) {
                added.push(cell);
            }
        }
        current + added.len() <= MAX_UNSAFE_TILES
    }

    pub fn can_block_without_disconnect(&self, cell: IVec2) -> bool {
        let neighbors: Vec<_> = [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y]
            .into_iter()
            .map(|direction| cell + direction)
            .filter(|neighbor| self.ground(*neighbor).is_some_and(Ground::walkable))
            .collect();
        let Some(&first) = neighbors.first() else {
            return true;
        };
        let mut visited = vec![first];
        let mut queue = VecDeque::from([first]);
        while let Some(current) = queue.pop_front() {
            for direction in [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
                let next = current + direction;
                if next != cell
                    && self.ground(next).is_some_and(Ground::walkable)
                    && !visited.contains(&next)
                {
                    visited.push(next);
                    queue.push_back(next);
                }
            }
        }
        neighbors.iter().all(|neighbor| visited.contains(neighbor))
    }

    pub fn make_hole(&mut self, cell: IVec2) -> bool {
        if !self.can_create_unsafe(&[cell]) {
            self.terrain_limit_hit = true;
            return false;
        }
        let index = Self::index(cell).expect("hole cell must be in bounds");
        let changed = self.tiles[index] != Ground::Hole;
        self.wet[index] = 0;
        self.remove_oil(cell);
        self.thaw(cell);
        self.acid[index] = None;
        self.clear_poison(cell);
        self.thorns.dissolve(&[cell]);
        self.set(cell, Ground::Hole);
        self.new_holes.push(cell);
        if changed {
            self.new_unsafe.push(cell);
        }
        true
    }

    pub fn make_ridge(&mut self, cell: IVec2) -> bool {
        if !self.can_create_unsafe(&[cell]) {
            self.terrain_limit_hit = true;
            return false;
        }
        let changed = self.ground(cell) != Some(Ground::Ridge);
        self.thaw(cell);
        self.dry(cell);
        self.absorb_acid(cell);
        self.set(cell, Ground::Ridge);
        if changed {
            self.new_unsafe.push(cell);
        }
        true
    }

    pub fn take_terrain_events(&mut self) -> (Vec<IVec2>, bool) {
        (
            std::mem::take(&mut self.new_unsafe),
            std::mem::take(&mut self.terrain_limit_hit),
        )
    }

    pub fn take_new_holes(&mut self) -> Vec<IVec2> {
        std::mem::take(&mut self.new_holes)
    }

    pub fn start_terrain_growth(&mut self, cell: IVec2, seed: u64, kind: TerrainGrowthKind) {
        self.terrain_growth
            .push((kind, VoidGrowth::new(cell, seed)));
    }

    pub fn terrain_growth_cells(&self) -> impl Iterator<Item = IVec2> + '_ {
        self.terrain_growth
            .iter()
            .flat_map(|(_, growth)| growth.cells())
            .chain(self.visual_growth.iter().flat_map(|growth| {
                growth.iter().enumerate().filter_map(|(index, kind)| {
                    kind.map(|_| IVec2::new((index % COLUMNS) as i32, (index / COLUMNS) as i32))
                })
            }))
    }

    pub fn terrain_growth_kind_at(&self, cell: IVec2) -> Option<TerrainGrowthKind> {
        self.terrain_growth
            .iter()
            .find_map(|(kind, growth)| growth.cells().any(|part| part == cell).then_some(*kind))
            .or_else(|| {
                self.visual_growth
                    .as_ref()
                    .and_then(|growth| Self::index(cell).and_then(|index| growth[index]))
            })
    }

    pub fn advance_terrain_growth(&mut self) {
        let mut outcomes = Vec::new();
        self.terrain_growth.retain_mut(|(kind, growth)| {
            if let Some(outcome) = growth.advance() {
                outcomes.push((*kind, outcome));
                false
            } else {
                true
            }
        });
        for (kind, cells) in outcomes {
            match kind {
                TerrainGrowthKind::Void => {
                    for cell in cells {
                        self.make_hole(cell);
                    }
                }
                TerrainGrowthKind::Stone => {
                    for cell in cells {
                        self.make_ridge(cell);
                    }
                }
            }
        }
    }

    pub fn add_acid(&mut self, cell: IVec2) {
        let index = Self::index(cell).expect("acid cell must be in bounds");
        let permanent = self.is_water(cell);
        let acid = self.acid[index].get_or_insert(Acid {
            permanent,
            turns: ACID_TURNS,
            fresh: true,
            electric_turns: 0,
            electric_fresh: false,
        });
        acid.permanent |= permanent;
        acid.turns = ACID_TURNS;
        acid.fresh = true;
    }

    pub fn absorb_acid(&mut self, cell: IVec2) {
        let index = Self::index(cell).expect("acid cell must be in bounds");
        if let Some(acid) = &mut self.acid[index] {
            acid.permanent = false;
            acid.turns = ACID_TURNS;
            acid.fresh = true;
        }
    }

    pub fn electrify_acid(&mut self, cell: IVec2) {
        let index = Self::index(cell).expect("acid cell must be in bounds");
        if let Some(acid) = &mut self.acid[index] {
            acid.electric_turns = ELECTRIFIED_ACID_TURNS;
            acid.electric_fresh = true;
        }
    }

    pub fn finish_thorn_turn(&mut self) -> (u32, u32) {
        let mut thorns = std::mem::take(&mut self.thorns);
        let returned = thorns.finish_turn(self);
        self.thorns = thorns;
        returned
    }

    pub fn is_frozen(&self, cell: IVec2) -> bool {
        Self::index(cell).is_some_and(|index| self.frozen[index])
    }

    pub fn freeze_water(&mut self, cell: IVec2) -> bool {
        if !self.is_water(cell) {
            return false;
        }
        let index = Self::index(cell).expect("water cell must be in bounds");
        self.frozen[index] = true;
        true
    }

    pub fn thaw(&mut self, cell: IVec2) {
        self.frozen[Self::index(cell).expect("board cell must be in bounds")] = false;
    }

    pub fn is_poisoned(&self, cell: IVec2) -> bool {
        Self::index(cell)
            .is_some_and(|index| self.poison.iter().any(|wave| wave.remaining[index] > 0))
    }

    pub fn clear_poison(&mut self, cell: IVec2) {
        let index = Self::index(cell).expect("poison cell must be in bounds");
        for wave in &mut self.poison {
            wave.remaining[index] = 0;
        }
        self.poison
            .retain(|wave| wave.remaining.iter().any(|remaining| *remaining > 0));
    }

    pub fn add_poison(&mut self, cell: IVec2) {
        let index = Self::index(cell).expect("board cell must be in bounds");
        let mut wave = PoisonWave {
            remaining: [0; COLUMNS * ROWS],
            visited: [false; COLUMNS * ROWS],
            fresh: true,
        };
        wave.remaining[index] = POISON_TURNS;
        wave.visited[index] = true;
        self.poison.push(wave);
    }

    fn advance_poison(&mut self, watered_poison: impl IntoIterator<Item = IVec2>) {
        let water: [bool; COLUMNS * ROWS] = std::array::from_fn(|index| {
            let cell = IVec2::new((index % COLUMNS) as i32, (index / COLUMNS) as i32);
            self.is_water(cell) && !self.is_frozen(cell)
        });
        let mut watered = [false; COLUMNS * ROWS];
        for cell in watered_poison {
            watered[Self::index(cell).expect("board cell must be in bounds")] = true;
        }
        for wave in &mut self.poison {
            if wave.fresh {
                wave.fresh = false;
                continue;
            }
            let previous = wave.remaining;
            for (index, remaining) in previous.into_iter().enumerate() {
                if remaining == 0 {
                    continue;
                }
                if self.frozen[index] || watered[index] {
                    continue;
                }
                if water[index] {
                    let cell = IVec2::new((index % COLUMNS) as i32, (index / COLUMNS) as i32);
                    for x in -1..=1 {
                        for y in -1..=1 {
                            if x == 0 && y == 0 {
                                continue;
                            }
                            let next = cell + IVec2::new(x, y);
                            if let Some(next_index) = Self::index(next)
                                && water[next_index]
                                && !wave.visited[next_index]
                            {
                                wave.visited[next_index] = true;
                                wave.remaining[next_index] = POISON_TURNS;
                            }
                        }
                    }
                }
                wave.remaining[index] -= 1;
            }
        }
        self.poison
            .retain(|wave| wave.remaining.iter().any(|remaining| *remaining > 0));
    }

    pub fn finish_turn(
        &mut self,
        refreshed_water: impl IntoIterator<Item = IVec2>,
        watered_poison: impl IntoIterator<Item = IVec2>,
    ) {
        self.advance_poison(watered_poison);
        for slot in &mut self.acid {
            if let Some(acid) = slot {
                if acid.fresh {
                    acid.fresh = false;
                } else if !acid.permanent {
                    acid.turns = acid.turns.saturating_sub(1);
                }
                if acid.electric_fresh {
                    acid.electric_fresh = false;
                } else {
                    acid.electric_turns = acid.electric_turns.saturating_sub(1);
                }
                if !acid.permanent && acid.turns == 0 {
                    *slot = None;
                }
            }
        }
        let mut protected_water = [false; COLUMNS * ROWS];
        for cell in refreshed_water {
            protected_water[Self::index(cell).expect("board cell must be in bounds")] = true;
        }
        for (index, water) in self.wet.iter_mut().enumerate() {
            if !protected_water[index] && !self.frozen[index] {
                *water = water.saturating_sub(1);
            }
        }
        for index in 0..self.wet.len() {
            if self.wet[index] == 0 && self.tiles[index] == Ground::Solid {
                self.expire_life_essence(index);
            }
        }
    }

    pub fn connected_water(&self, start: IVec2) -> Vec<IVec2> {
        self.connected_tiles(start, |cell| self.is_water(cell))
    }

    pub fn connected_oil(&self, start: IVec2) -> Vec<IVec2> {
        self.connected_tiles(start, |cell| self.has_oil(cell))
    }

    fn connected_tiles(&self, start: IVec2, matches: impl Fn(IVec2) -> bool) -> Vec<IVec2> {
        let Some(start_index) = Self::index(start) else {
            return Vec::new();
        };
        if !matches(start) {
            return Vec::new();
        }
        let mut visited = [false; COLUMNS * ROWS];
        let mut queue = VecDeque::from([start]);
        let mut connected = Vec::new();
        visited[start_index] = true;
        while let Some(cell) = queue.pop_front() {
            connected.push(cell);
            for x in -1..=1 {
                for y in -1..=1 {
                    if x == 0 && y == 0 {
                        continue;
                    }
                    let next = cell + IVec2::new(x, y);
                    if let Some(index) = Self::index(next)
                        && !visited[index]
                        && matches(next)
                    {
                        visited[index] = true;
                        queue.push_back(next);
                    }
                }
            }
        }
        connected
    }

    fn index(cell: IVec2) -> Option<usize> {
        if cell.x < 0 || cell.y < 0 || cell.x >= COLUMNS as i32 || cell.y >= ROWS as i32 {
            return None;
        }
        Some(cell.y as usize * COLUMNS + cell.x as usize)
    }

    pub fn set(&mut self, cell: IVec2, ground: Ground) {
        let index = Self::index(cell).expect("board cell must be in bounds");
        if matches!(ground, Ground::Hole | Ground::Ridge) {
            self.thaw(cell);
        }
        self.tiles[index] = ground;
        if !self.is_water(cell) {
            self.expire_life_essence(index);
        }
    }

    fn walkable_connected(&self) -> bool {
        let mut visited = [false; COLUMNS * ROWS];
        let mut queue = VecDeque::from([PLAYER_STARTS[0]]);
        visited[Self::index(PLAYER_STARTS[0]).unwrap()] = true;
        while let Some(cell) = queue.pop_front() {
            for delta in [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
                let next = cell + delta;
                if let Some(index) = Self::index(next)
                    && !visited[index]
                    && self.tiles[index].walkable()
                {
                    visited[index] = true;
                    queue.push_back(next);
                }
            }
        }
        self.tiles
            .iter()
            .enumerate()
            .all(|(index, tile)| !tile.walkable() || visited[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_rand::prelude::WyRand;
    use rand::SeedableRng;

    #[test]
    fn replacing_frozen_water_with_unsafe_terrain_clears_ice() {
        let cell = IVec2::new(1, 1);
        for ground in [Ground::Hole, Ground::Ridge] {
            for natural_water in [false, true] {
                let mut board = Board::default();
                if natural_water {
                    board.make_water(cell);
                } else {
                    board.make_wet(cell);
                }
                assert!(board.freeze_water(cell));
                board.set(cell, ground);
                assert_eq!(board.ground(cell), Some(ground));
                assert!(!board.is_frozen(cell));
                assert!(!board.freeze_water(cell));
            }
        }
    }

    #[test]
    fn terrain_limit_preserves_frozen_water_when_creation_is_blocked() {
        let mut board = Board::default();
        let cells: Vec<_> = (0..ROWS)
            .flat_map(|y| (0..COLUMNS).map(move |x| IVec2::new(x as i32, y as i32)))
            .collect();
        for &cell in &cells[..MAX_UNSAFE_TILES] {
            board.set(cell, Ground::Hole);
        }
        let cell = cells[MAX_UNSAFE_TILES];
        board.make_water(cell);
        assert!(board.freeze_water(cell));
        assert!(!board.make_hole(cell));
        assert!(!board.make_ridge(cell));
        assert_eq!(board.ground(cell), Some(Ground::Water));
        assert!(board.is_frozen(cell));
    }

    #[test]
    fn weather_does_not_block_the_only_path_between_walkable_tiles() {
        let mut board = Board::default();
        for y in 0..ROWS {
            for x in 0..COLUMNS {
                board.set(IVec2::new(x as i32, y as i32), Ground::Hole);
            }
        }
        for cell in [IVec2::new(1, 1), IVec2::new(2, 1), IVec2::new(3, 1)] {
            board.set(cell, Ground::Solid);
        }
        assert!(!board.can_block_without_disconnect(IVec2::new(2, 1)));
        assert!(board.can_block_without_disconnect(IVec2::new(1, 1)));
        for cell in [IVec2::new(1, 2), IVec2::new(2, 2), IVec2::new(3, 2)] {
            board.set(cell, Ground::Solid);
        }
        assert!(board.can_block_without_disconnect(IVec2::new(2, 1)));
    }

    #[test]
    fn generated_boards_stay_connected_and_keep_starts_clear() {
        for seed in 0..200 {
            let board = Board::generate(&mut WyRand::seed_from_u64(seed));
            assert!(board.walkable_connected(), "seed {seed}");
            assert_eq!(
                board
                    .tiles
                    .iter()
                    .filter(|tile| **tile == Ground::Ridge)
                    .count(),
                RIDGE_COUNT
            );
            assert_eq!(
                board
                    .tiles
                    .iter()
                    .filter(|tile| **tile == Ground::Water)
                    .count(),
                WATER_COUNT
            );
            assert!(!board.tiles.contains(&Ground::Hole));
            for start in PLAYER_STARTS {
                assert_eq!(board.ground(start), Some(Ground::Solid));
                for x in -1..=1 {
                    for y in -1..=1 {
                        assert!(board.ground(start + IVec2::new(x, y)).unwrap().walkable());
                    }
                }
            }
        }
    }

    #[test]
    fn different_seeds_produce_different_layouts() {
        let first = Board::generate(&mut WyRand::seed_from_u64(0));
        let second = Board::generate(&mut WyRand::seed_from_u64(1));
        assert_ne!(first.tiles, second.tiles);
    }

    #[test]
    fn poison_spreads_one_step_per_turn_and_clears_behind_the_wave() {
        let mut board = Board::default();
        let cells = [
            IVec2::new(1, 1),
            IVec2::new(2, 1),
            IVec2::new(3, 1),
            IVec2::new(4, 1),
        ];
        for cell in cells {
            board.set(cell, Ground::Water);
        }
        board.add_poison(cells[0]);
        for expected in [
            [true, false, false, false],
            [true, true, false, false],
            [false, true, true, false],
            [false, false, true, true],
            [false, false, false, true],
            [false, false, false, false],
        ] {
            board.finish_turn(None, None);
            assert_eq!(cells.map(|cell| board.is_poisoned(cell)), expected);
        }
        assert!(board.poison.is_empty());
    }

    #[test]
    fn poison_spreads_diagonally_but_does_not_cycle_or_cross_dry_ground() {
        let mut board = Board::default();
        let source = IVec2::new(1, 1);
        let diagonal = IVec2::new(2, 2);
        let connected = IVec2::new(3, 2);
        let isolated = IVec2::new(5, 2);
        for cell in [source, diagonal, connected, isolated] {
            board.set(cell, Ground::Water);
        }
        board.add_poison(source);
        board.finish_turn(None, None);
        board.finish_turn(None, None);
        assert!(board.is_poisoned(source));
        assert!(board.is_poisoned(diagonal));
        assert!(!board.is_poisoned(connected));
        board.finish_turn(None, None);
        assert!(!board.is_poisoned(source));
        assert!(board.is_poisoned(diagonal));
        assert!(board.is_poisoned(connected));
        for _ in 0..4 {
            board.finish_turn(None, None);
            assert!(!board.is_poisoned(source));
            assert!(!board.is_poisoned(isolated));
        }
        assert!(board.poison.is_empty());
    }

    #[test]
    fn wetting_poison_about_to_expire_lets_it_spread_next_turn() {
        let mut board = Board::default();
        let source = IVec2::new(1, 1);
        let neighbor = IVec2::new(2, 1);
        board.set(neighbor, Ground::Water);
        board.add_poison(source);
        board.finish_turn(None, None);
        board.finish_turn(None, None);
        assert!(board.is_poisoned(source));
        assert!(!board.is_poisoned(neighbor));
        board.make_wet(source);
        board.finish_turn(Some(source), Some(source));
        assert!(board.is_poisoned(source));
        assert!(!board.is_poisoned(neighbor));
        board.finish_turn(None, None);
        assert!(!board.is_poisoned(source));
        assert!(board.is_poisoned(neighbor));
    }

    #[test]
    fn poison_cannot_spread_into_frozen_water() {
        let mut board = Board::default();
        let source = IVec2::new(1, 1);
        let frozen = IVec2::new(2, 1);
        let beyond = IVec2::new(3, 1);
        for cell in [source, frozen, beyond] {
            board.set(cell, Ground::Water);
        }
        assert!(board.freeze_water(frozen));
        board.add_poison(source);
        for _ in 0..4 {
            board.finish_turn(None, None);
            assert!(!board.is_poisoned(frozen));
            assert!(!board.is_poisoned(beyond));
        }
        assert!(!board.is_poisoned(source));
    }

    #[test]
    fn frozen_poison_keeps_its_duration_and_spreads_after_thaw() {
        let mut board = Board::default();
        let source = IVec2::new(1, 1);
        let neighbor = IVec2::new(2, 1);
        board.set(source, Ground::Water);
        board.set(neighbor, Ground::Water);
        board.add_poison(source);
        board.finish_turn(None, None);
        assert!(board.freeze_water(source));
        for _ in 0..4 {
            board.finish_turn(None, None);
            assert!(board.is_poisoned(source));
            assert!(!board.is_poisoned(neighbor));
        }
        board.thaw(source);
        board.finish_turn(None, None);
        assert!(board.is_poisoned(source));
        assert!(board.is_poisoned(neighbor));
        board.finish_turn(None, None);
        assert!(!board.is_poisoned(source));
        assert!(board.is_poisoned(neighbor));
    }

    #[test]
    fn poison_can_spread_into_a_tile_that_thaws_while_the_source_is_active() {
        let mut board = Board::default();
        let source = IVec2::new(1, 1);
        let neighbor = IVec2::new(2, 1);
        board.set(source, Ground::Water);
        board.set(neighbor, Ground::Water);
        assert!(board.freeze_water(neighbor));
        board.add_poison(source);
        board.finish_turn(None, None);
        board.finish_turn(None, None);
        assert!(!board.is_poisoned(neighbor));
        board.thaw(neighbor);
        board.finish_turn(None, None);
        assert!(board.is_poisoned(neighbor));
    }
}
