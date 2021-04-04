use crate::{
    cache::{buf::Buffer, index::CacheIndex, indextype::IndexType},
    types::coordinate::Coordinate,
    utils::error::CacheResult,
};

use serde::Serialize;

use std::{
    collections::HashMap,
    convert::TryInto,
    fs::{self, File},
    io::Write,
    iter,
};

pub struct WorldMapFileType;

impl WorldMapFileType {
    #![allow(missing_docs)]
    pub const ZONES: u32 = 0;
    pub const PASTES: u32 = 1;

    pub const SMALL: u32 = 2;

    pub const UNKNOWN_3: u32 = 3;
    pub const BIG: u32 = 4;
}

#[derive(Debug, Serialize)]
pub struct MapZone {
    id: u32,
    internal_name: String,
    name: String,
    center: Coordinate,
    unknown_1: u32,
    show: bool,
    default_zoom: u8,
    unknown_2: u8,
    bounds: Vec<BoundDef>,
}

impl MapZone {
    pub fn dump_all() -> CacheResult<HashMap<u32, Self>> {
        Ok(CacheIndex::new(IndexType::WORLDMAP)?
            .archive(WorldMapFileType::ZONES)?
            .take_files()
            .into_iter()
            .map(|(file_id, file)| (file_id, Self::deserialize(file_id, file)))
            .collect())
    }

    fn deserialize(id: u32, file: Vec<u8>) -> Self {
        let mut buf = Buffer::new(file);
        let internal_name = buf.read_string();
        let name = buf.read_string();
        let center = buf.read_unsigned_int().try_into().unwrap();
        let unknown_1 = buf.read_unsigned_int();
        let show = match buf.read_unsigned_byte() {
            0 => false,
            1 => true,
            other => unimplemented!("Cannot convert value {} for 'show' to boolean", other),
        };
        let default_zoom = buf.read_unsigned_byte();
        let unknown_2 = buf.read_unsigned_byte();
        let count = buf.read_unsigned_byte() as usize;
        let bounds = iter::repeat_with(|| BoundDef::deserialize(&mut buf)).take(count).collect();

        debug_assert_eq!(buf.remaining(), 0);

        Self {
            id,
            internal_name,
            name,
            center,
            unknown_1,
            show,
            default_zoom,
            unknown_2,
            bounds,
        }
    }

    /// Get a reference to the map zone's internal name.
    pub fn internal_name(&self) -> &str {
        &self.internal_name
    }

    /// Get a reference to the map zone's internal name.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn center(&self) -> Coordinate {
        self.center
    }

    /// Get a reference to the map zone's show.
    pub const fn show(&self) -> bool {
        self.show
    }

    /// Get a reference to the map zone's unknown 1.
    pub const fn unknown_1(&self) -> u32 {
        self.unknown_1
    }

    /// Get a reference to the map zone's default zoom.
    pub const fn default_zoom(&self) -> u8 {
        self.default_zoom
    }

    /// Get a reference to the map zone's unknown 2.
    pub const fn unknown_2(&self) -> u8 {
        self.unknown_2
    }

    /// Get a reference to the map zone's bounds.
    pub fn bounds(&self) -> &[BoundDef] {
        self.bounds.as_slice()
    }
}

mod mapzone_fields_impl {
    use crate::cache::buf::Buffer;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub struct BoundDef {
        plane: u8,
        src: Bound,
        dst: Bound,
    }

    impl BoundDef {
        pub fn deserialize(buf: &mut Buffer) -> Self {
            let plane = buf.read_unsigned_byte();
            let src = Bound::deserialize(buf);
            let dst = Bound::deserialize(buf);
            Self { plane, src, dst }
        }
    }

    /// Represents a rectangular area of the game map..
    #[derive(Debug, Serialize)]
    pub struct Bound {
        pub west: u16,
        pub south: u16,
        pub east: u16,
        pub north: u16,
    }

    impl Bound {
        pub fn deserialize(buf: &mut Buffer) -> Self {
            let west = buf.read_unsigned_short();
            let south = buf.read_unsigned_short();
            let east = buf.read_unsigned_short();
            let north = buf.read_unsigned_short();

            Self { west, south, east, north }
        }
    }
}

pub use mapzone_fields_impl::*;

#[derive(Debug, Serialize)]
pub struct MapPastes {
    id: u32,
    dim_i: u8,
    dim_j: u8,
    pastes: Vec<Paste>,
}

impl MapPastes {
    pub fn dump_all() -> CacheResult<HashMap<u32, Self>> {
        Ok(CacheIndex::new(IndexType::WORLDMAP)?
            .archive(WorldMapFileType::PASTES)?
            .take_files()
            .into_iter()
            .map(|(file_id, file)| (file_id, Self::deserialize(file_id, file)))
            .collect())
    }
    fn deserialize(id: u32, file: Vec<u8>) -> Self {
        let mut buf = Buffer::new(file);
        let mut pastes = Vec::new();

        let square_count = buf.read_unsigned_short() as usize;
        let square_pastes = iter::repeat_with(|| Paste::deserialize_square(&mut buf)).take(square_count);
        pastes.extend(square_pastes);

        let chunk_count = buf.read_unsigned_short() as usize;
        let chunk_pastes = iter::repeat_with(|| Paste::deserialize_chunk(&mut buf)).take(chunk_count);
        pastes.extend(chunk_pastes);
        let dim_i = buf.read_unsigned_byte();
        let dim_j = buf.read_unsigned_byte();
        debug_assert_eq!(buf.remaining(), 0);

        Self { id, dim_i, dim_j, pastes }
    }
}

#[derive(Debug, Serialize)]
pub struct Paste {
    pub src_plane: u8,
    pub n_planes: u8,
    pub src_i: u16,
    pub src_j: u16,
    pub src_chunk: Option<Chunk>,

    pub dst_plane: u8,
    pub dst_i: u16,
    pub dst_j: u16,

    pub dst_chunk: Option<Chunk>,
}

impl Paste {
    fn deserialize_square(buf: &mut Buffer) -> Self {
        let src_plane = buf.read_unsigned_byte();
        let n_planes = buf.read_unsigned_byte();
        let src_i = buf.read_unsigned_short();
        let src_j = buf.read_unsigned_short();

        let dst_plane = buf.read_unsigned_byte();
        let dst_i = buf.read_unsigned_short();
        let dst_j = buf.read_unsigned_short();

        Self {
            src_plane,
            n_planes,
            src_i,
            src_j,
            src_chunk: None,

            dst_plane,
            dst_i,
            dst_j,

            dst_chunk: None,
        }
    }

    fn deserialize_chunk(buf: &mut Buffer) -> Self {
        let src_plane = buf.read_unsigned_byte();
        let n_planes = buf.read_unsigned_byte();
        let src_i = buf.read_unsigned_short();
        let src_j = buf.read_unsigned_short();
        let src_chunk = Chunk::deserialize(buf);

        let dst_plane = buf.read_unsigned_byte();
        let dst_i = buf.read_unsigned_short();
        let dst_j = buf.read_unsigned_short();
        let dst_chunk = Chunk::deserialize(buf);

        Self {
            src_plane,
            n_planes,
            src_i,
            src_j,
            src_chunk: Some(src_chunk),

            dst_plane,
            dst_i,
            dst_j,

            dst_chunk: Some(dst_chunk),
        }
    }
}

mod mappaste_fields_impl {
    use crate::cache::buf::Buffer;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub struct Chunk {
        pub x: u8,
        pub y: u8,
    }

    impl Chunk {
        pub fn deserialize(buf: &mut Buffer) -> Self {
            let x = buf.read_unsigned_byte();
            let y = buf.read_unsigned_byte();
            Self { x, y }
        }
    }
}
pub use mappaste_fields_impl::*;

pub fn export_pastes() -> CacheResult<()> {
    use std::collections::BTreeMap;

    fs::create_dir_all("out")?;
    // btreemap has deterministic order
    let map_zones: BTreeMap<u32, MapPastes> = MapPastes::dump_all()?.into_iter().collect();

    let mut file = File::create("out/map_pastes.json")?;
    let data = serde_json::to_string_pretty(&map_zones)?;
    file.write_all(data.as_bytes())?;
    Ok(())
}

pub fn export_zones() -> CacheResult<()> {
    fs::create_dir_all("out")?;
    let mut map_zones = MapZone::dump_all()?.into_values().collect::<Vec<_>>();
    map_zones.sort_unstable_by_key(|loc| loc.id);

    let mut file = File::create("out/map_zones.json")?;
    let data = serde_json::to_string_pretty(&map_zones)?;
    file.write_all(data.as_bytes())?;
    Ok(())
}

pub fn dump_small() -> CacheResult<()> {
    fs::create_dir_all("out/world_map_small")?;

    let files = CacheIndex::new(IndexType::WORLDMAP)?.archive(WorldMapFileType::SMALL)?.take_files();
    for (id, data) in files {
        let filename = format!("out/world_map_small/{}.png", id);
        let mut file = File::create(filename)?;
        file.write_all(&data)?;
    }

    Ok(())
}

pub fn dump_big() -> CacheResult<()> {
    fs::create_dir_all("out/world_map_big")?;

    let files = CacheIndex::new(IndexType::WORLDMAP)?.archive(WorldMapFileType::BIG)?.take_files();

    for (id, data) in files {
        let mut buf = Buffer::new(data);
        let size = buf.read_unsigned_int() as usize;
        let img = buf.read_n_bytes(size);

        let filename = format!("out/world_map_big/{}.png", id);
        let mut file = File::create(filename)?;
        file.write_all(&img)?;
    }

    Ok(())
}

#[cfg(test)]
mod worldmap_tests {
    use super::*;

    #[test]
    #[ignore]
    fn check_pastes() -> CacheResult<()> {
        export_pastes()
    }

    #[test]
    #[ignore]
    fn check_zones() -> CacheResult<()> {
        export_zones()
    }
    #[test]
    #[ignore]
    fn check_big() -> CacheResult<()> {
        dump_big()
    }

    #[test]
    #[ignore]
    fn check_small() -> CacheResult<()> {
        dump_small()
    }
}
