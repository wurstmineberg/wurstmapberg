#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
        cmp::Ordering::*,
        collections::{
            BTreeSet,
            HashMap,
        },
        path::{
            Path,
            PathBuf,
        },
        pin::pin,
        sync::Arc,
    },
    chrono::prelude::*,
    futures::stream::{
        FuturesUnordered,
        TryStreamExt as _,
    },
    image::{
        Rgba,
        RgbaImage,
    },
    mcanvil::{
        Dimension,
        Region,
        RegionDecodeError,
    },
    parking_lot::Mutex,
    wheel::fs,
};

mod colors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MapColor {
    Clear,
    PaleGreen,
    PaleYellow,
    WhiteGray,
    BrightRed,
    PalePurple,
    IronGray,
    DarkGreen,
    White,
    LightBlueGray,
    DirtBrown,
    StoneGray,
    WaterBlue,
    OakTan,
    OffWhite,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
    Gold,
    DiamondBlue,
    LapisBlue,
    EmeraldGreen,
    SpruceBrown,
    DarkRed,
    TerracottaWhite,
    TerracottaOrange,
    TerracottaMagenta,
    TerracottaLightBlue,
    TerracottaYellow,
    TerracottaLime,
    TerracottaPink,
    TerracottaGray,
    TerracottaLightGray,
    TerracottaCyan,
    TerracottaPurple,
    TerracottaBlue,
    TerracottaBrown,
    TerracottaGreen,
    TerracottaRed,
    TerracottaBlack,
    DullRed,
    DullPink,
    DarkCrimson,
    Teal,
    DarkAqua,
    DarkDullPink,
    BrightTeal,
    DeepslateGray,
    RawIronPink,
    LichenGreen,
}

enum Tint {
    Dark,
    Normal,
    Light,
}

impl Tint {
    fn multiplier(&self) -> u16 {
        match self {
            Self::Dark => 180,
            Self::Normal => 220,
            Self::Light => 255,
        }
    }
}

impl MapColor {
    fn tint(&self, tint: Tint) -> Rgba<u8> {
        let base_rgb = match self {
            MapColor::Clear => return Rgba([0; 4]),
            MapColor::PaleGreen => 8368696_u32,
            MapColor::PaleYellow => 16247203,
            MapColor::WhiteGray => 13092807,
            MapColor::BrightRed => 16711680,
            MapColor::PalePurple => 10526975,
            MapColor::IronGray => 10987431,
            MapColor::DarkGreen => 31744,
            MapColor::White => 16777215,
            MapColor::LightBlueGray => 10791096,
            MapColor::DirtBrown => 9923917,
            MapColor::StoneGray => 7368816,
            MapColor::WaterBlue => 4210943,
            MapColor::OakTan => 9402184,
            MapColor::OffWhite => 16776437,
            MapColor::Orange => 14188339,
            MapColor::Magenta => 11685080,
            MapColor::LightBlue => 6724056,
            MapColor::Yellow => 15066419,
            MapColor::Lime => 8375321,
            MapColor::Pink => 15892389,
            MapColor::Gray => 5000268,
            MapColor::LightGray => 10066329,
            MapColor::Cyan => 5013401,
            MapColor::Purple => 8339378,
            MapColor::Blue => 3361970,
            MapColor::Brown => 6704179,
            MapColor::Green => 6717235,
            MapColor::Red => 10040115,
            MapColor::Black => 1644825,
            MapColor::Gold => 16445005,
            MapColor::DiamondBlue => 6085589,
            MapColor::LapisBlue => 4882687,
            MapColor::EmeraldGreen => 55610,
            MapColor::SpruceBrown => 8476209,
            MapColor::DarkRed => 7340544,
            MapColor::TerracottaWhite => 13742497,
            MapColor::TerracottaOrange => 10441252,
            MapColor::TerracottaMagenta => 9787244,
            MapColor::TerracottaLightBlue => 7367818,
            MapColor::TerracottaYellow => 12223780,
            MapColor::TerracottaLime => 6780213,
            MapColor::TerracottaPink => 10505550,
            MapColor::TerracottaGray => 3746083,
            MapColor::TerracottaLightGray => 8874850,
            MapColor::TerracottaCyan => 5725276,
            MapColor::TerracottaPurple => 8014168,
            MapColor::TerracottaBlue => 4996700,
            MapColor::TerracottaBrown => 4993571,
            MapColor::TerracottaGreen => 5001770,
            MapColor::TerracottaRed => 9321518,
            MapColor::TerracottaBlack => 2430480,
            MapColor::DullRed => 12398641,
            MapColor::DullPink => 9715553,
            MapColor::DarkCrimson => 6035741,
            MapColor::Teal => 1474182,
            MapColor::DarkAqua => 3837580,
            MapColor::DarkDullPink => 5647422,
            MapColor::BrightTeal => 1356933,
            MapColor::DeepslateGray => 6579300,
            MapColor::RawIronPink => 14200723,
            MapColor::LichenGreen => 8365974,
        };
        let [_, r, g, b] = base_rgb.to_be_bytes().map(|channel| (u16::from(channel) * tint.multiplier() / 255) as u8);
        Rgba([r, g, b, u8::MAX])
    }
}

#[derive(Debug, Clone, Copy)]
enum BlockMapColor {
    Single(MapColor),
    Bed {
        head: MapColor,
        foot: MapColor,
    },
    Crops {
        growing: MapColor,
        grown: MapColor,
    },
    Pillar {
        top: MapColor,
        side: MapColor,
    },
    Waterloggable {
        dry: MapColor,
        wet: MapColor,
    },
}

const DIMENSION: Dimension = Dimension::Overworld;

static FALLBACK_HEIGHTMAP: &[[i32; 16]; 16] = &[[320; 16]; 16];

#[derive(clap::Parser)]
#[clap(version)]
struct Args {
    world_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] Image(#[from] image::ImageError),
    #[error(transparent)] Task(#[from] tokio::task::JoinError),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    /// Note these are keyed by region coords, not chunk coords
    #[error("{}", .0.values().next().unwrap())]
    Cols(HashMap<[i32; 2], mcanvil::ChunkColumnDecodeError>),
    #[error("failed to get list of regions: {0}")]
    ListRegions(RegionDecodeError),
    #[error("a region that was listed has since been deleted")]
    RegionNotFound,
    #[error("{}", .0.values().next().unwrap())]
    Regions(HashMap<[i32; 2], RegionDecodeError>),
}

#[wheel::main(max_blocking_threads = 0)]
async fn main(Args { world_dir }: Args) -> Result<(), Error> {
    let block_colors = Arc::new(colors::get_block_colors());
    fs::create_dir_all("out").await?;
    let region_errors = Arc::<Mutex<HashMap<_, _>>>::default();
    let col_errors = Arc::<Mutex<HashMap<_, _>>>::default();
    let mut coords = HashMap::<_, BTreeSet<_>>::default();
    let mut coords_stream = pin!(Region::all_coords(&world_dir, DIMENSION));
    while let Some([x, z]) = coords_stream.try_next().await.map_err(Error::ListRegions)? {
        coords.entry(x).or_default().insert(z);
    }
    let mut renderers = FuturesUnordered::default();
    for (x, zs) in coords {
        let block_colors = &block_colors;
        let region_errors = region_errors.clone();
        let col_errors = col_errors.clone();
        let world_dir = &world_dir;
        renderers.push(async move {
            let mut prev = None;
            for z in zs {
                let region = match Region::find(world_dir, DIMENSION, [x, z]).await {
                    Ok(Some(region)) => region,
                    Ok(None) => return Err(Error::RegionNotFound),
                    Err(e) => {
                        region_errors.lock().insert([x, z], e);
                        return Ok(())
                    }
                };
                let block_colors = block_colors.clone();
                let col_errors = col_errors.clone();
                prev = Some(tokio::task::spawn_blocking(move || {
                    println!("{} processing region {}, {}", Local::now().format("%F %T"), region.coords[0], region.coords[1]);
                    let mut region_img = RgbaImage::new(16 * 32, 16 * 32);
                    for col in &region {
                        let col = match col {
                            Ok(col) => col,
                            Err(e) => {
                                col_errors.lock().insert([x, z], e);
                                return Ok(region)
                            }
                        };
                        let heightmap = col.heightmaps.get("WORLD_SURFACE").unwrap_or_else(|| &FALLBACK_HEIGHTMAP);
                        for (block_z, row) in heightmap.iter().enumerate() {
                            for (block_x, max_y) in row.iter().enumerate() {
                                let mut col_color = MapColor::Clear;
                                let mut y = *max_y;
                                while y >= col.y_pos {
                                    let chunk_y = y.div_euclid(16) as i8;
                                    let block_y = y.rem_euclid(16) as usize;
                                    if let Some(chunk) = col.section_at(chunk_y) {
                                        let block = &chunk.block_relative([block_x as u8, block_y as u8, block_z as u8]);
                                        let Some(&color) = block_colors.get(&block.name) else { continue };
                                        col_color = match color {
                                            BlockMapColor::Single(color) => color,
                                            BlockMapColor::Bed { head, foot } => if block.properties.get("part").is_some_and(|part| part == "head") { head } else { foot },
                                            BlockMapColor::Crops { growing, grown } => if block.properties.get("age").is_some_and(|age| age == "7") { grown } else { growing },
                                            BlockMapColor::Pillar { top, side } => if block.properties.get("axis").is_some_and(|axis| axis != "y") { side } else { top },
                                            BlockMapColor::Waterloggable { dry, wet } => if block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true") { wet } else { dry },
                                        };
                                        if col_color != MapColor::Clear { break }
                                    }
                                    if y == col.y_pos { break }
                                    y -= 1;
                                }
                                let x = col.x_pos * 16 + block_x as i32;
                                let z = col.z_pos * 16 + block_z as i32;
                                let tint = match col_color {
                                    MapColor::Clear => Tint::Normal,
                                    MapColor::WaterBlue => {
                                        let water_depth = (col.y_pos..=y).rev().take_while(|y| {
                                            let chunk_y = y.div_euclid(16) as i8;
                                            let block_y = y.rem_euclid(16) as usize;
                                            if let Some(chunk) = col.section_at(chunk_y) {
                                                let block = &chunk.block_relative([block_x as u8, block_y as u8, block_z as u8]);
                                                let Some(&color) = block_colors.get(&block.name) else { return false };
                                                col_color = match color {
                                                    BlockMapColor::Single(color) => color,
                                                    BlockMapColor::Bed { head, foot } => if block.properties.get("part").is_some_and(|part| part == "head") { head } else { foot },
                                                    BlockMapColor::Crops { growing, grown } => if block.properties.get("age").is_some_and(|age| age == "7") { grown } else { growing },
                                                    BlockMapColor::Pillar { top, side } => if block.properties.get("axis").is_some_and(|axis| axis != "y") { side } else { top },
                                                    BlockMapColor::Waterloggable { dry, wet } => if block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true") { wet } else { dry },
                                                };
                                                col_color == MapColor::WaterBlue || block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true")
                                            } else {
                                                false
                                            }
                                        }).count();
                                        match water_depth {
                                            ..=2 => Tint::Light,
                                            3..=4 => if (block_x + block_z) % 2 == 0 { Tint::Light } else { Tint::Normal },
                                            5..=6 => Tint::Normal,
                                            7..=9 => if (block_x + block_z) % 2 == 0 { Tint::Normal } else { Tint::Dark },
                                            _ => Tint::Dark,
                                        }
                                    }
                                    _ => {
                                        let north_neighbor = 'north_neighbor: {
                                            if let Some(block_z) = block_z.checked_sub(1) {
                                                // same chunk
                                                (col.y_pos..=heightmap[block_z][block_x]).rev().find(|y| {
                                                    let chunk_y = y.div_euclid(16) as i8;
                                                    let block_y = y.rem_euclid(16) as usize;
                                                    if let Some(chunk) = col.section_at(chunk_y) {
                                                        let block = &chunk.block_relative([block_x as u8, block_y as u8, block_z as u8]);
                                                        let Some(&color) = block_colors.get(&block.name) else { return false };
                                                        col_color = match color {
                                                            BlockMapColor::Single(color) => color,
                                                            BlockMapColor::Bed { head, foot } => if block.properties.get("part").is_some_and(|part| part == "head") { head } else { foot },
                                                            BlockMapColor::Crops { growing, grown } => if block.properties.get("age").is_some_and(|age| age == "7") { grown } else { growing },
                                                            BlockMapColor::Pillar { top, side } => if block.properties.get("axis").is_some_and(|axis| axis != "y") { side } else { top },
                                                            BlockMapColor::Waterloggable { dry, wet } => if block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true") { wet } else { dry },
                                                        };
                                                        col_color != MapColor::Clear
                                                    } else {
                                                        false
                                                    }
                                                })
                                            } else {
                                                // different chunk
                                                let north_region = if col.z_pos.rem_euclid(32) > 0 {
                                                    // same region
                                                    &region
                                                } else if let Some(prev) = &prev {
                                                    // different region
                                                    prev
                                                } else {
                                                    // not on map
                                                    break 'north_neighbor None
                                                };
                                                let col = match north_region.chunk_column([col.x_pos, col.z_pos - 1]) {
                                                    Ok(col) => col,
                                                    Err(e) => {
                                                        col_errors.lock().insert([x, z], e);
                                                        return Ok(region)
                                                    }
                                                };
                                                col.and_then(|col| {
                                                    let heightmap = col.heightmaps.get("WORLD_SURFACE").unwrap_or_else(|| &FALLBACK_HEIGHTMAP);
                                                    (col.y_pos..=heightmap[15][block_x]).rev().find(|y| {
                                                        let chunk_y = y.div_euclid(16) as i8;
                                                        let block_y = y.rem_euclid(16) as usize;
                                                        if let Some(chunk) = col.section_at(chunk_y) {
                                                            let block = &chunk.block_relative([block_x as u8, block_y as u8, 15]);
                                                            let Some(&color) = block_colors.get(&block.name) else { return false };
                                                            col_color = match color {
                                                                BlockMapColor::Single(color) => color,
                                                                BlockMapColor::Bed { head, foot } => if block.properties.get("part").is_some_and(|part| part == "head") { head } else { foot },
                                                                BlockMapColor::Crops { growing, grown } => if block.properties.get("age").is_some_and(|age| age == "7") { grown } else { growing },
                                                                BlockMapColor::Pillar { top, side } => if block.properties.get("axis").is_some_and(|axis| axis != "y") { side } else { top },
                                                                BlockMapColor::Waterloggable { dry, wet } => if block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true") { wet } else { dry },
                                                            };
                                                            col_color != MapColor::Clear
                                                        } else {
                                                            false
                                                        }
                                                    })
                                                })
                                            }
                                        }.unwrap_or(y);
                                        match y.cmp(&north_neighbor) {
                                            Less => Tint::Dark,
                                            Equal => Tint::Normal,
                                            Greater => Tint::Light,
                                        }
                                    }
                                };
                                region_img[(x.rem_euclid(16 * 32) as u32, z.rem_euclid(16 * 32) as u32)] = col_color.tint(tint);
                            }
                        }
                    }
                    region_img.save_with_format(Path::new("out").join(format!("r.{}.{}.png", region.coords[0], region.coords[1])), image::ImageFormat::Png)?; //TODO async
                    Ok::<_, Error>(region)
                }).await??);
            }
            Ok(())
        });
    }
    while let Some(()) = renderers.try_next().await? {}
    let region_errors = Arc::into_inner(region_errors).unwrap().into_inner();
    let col_errors = Arc::into_inner(col_errors).unwrap().into_inner();
    if !region_errors.is_empty() {
        Err(Error::Regions(region_errors))
    } else if !col_errors.is_empty() {
        Err(Error::Cols(col_errors))
    } else {
        Ok(())
    }
}
