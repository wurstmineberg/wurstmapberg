#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
        cmp::Ordering::*,
        collections::{
            BTreeSet,
            HashMap,
        },
        path::PathBuf,
        pin::pin,
        sync::Arc,
    },
    futures::stream::{
        FuturesUnordered,
        TryStreamExt as _,
    },
    image::{
        ImageError,
        Rgba,
        RgbaImage,
    },
    mcanvil::{
        Dimension,
        Region,
        RegionDecodeError,
    },
    parking_lot::Mutex,
    tokio::io,
    wheel::fs,
};

mod colors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MapColor {
    None,
    Grass,
    Sand,
    Wool,
    Fire,
    Ice,
    Metal,
    Plant,
    Snow,
    Clay,
    Dirt,
    Stone,
    Water,
    Wood,
    Quartz,
    ColorOrange,
    ColorMagenta,
    ColorLightBlue,
    ColorYellow,
    ColorLightGreen,
    ColorPink,
    ColorGray,
    ColorLightGray,
    ColorCyan,
    ColorPurple,
    ColorBlue,
    ColorBrown,
    ColorGreen,
    ColorRed,
    ColorBlack,
    Gold,
    Diamond,
    Lapis,
    Emerald,
    Podzol,
    Nether,
    TerracottaWhite,
    TerracottaOrange,
    TerracottaMagenta,
    TerracottaLightBlue,
    TerracottaYellow,
    TerracottaLightGreen,
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
    CrimsonNylium,
    CrimsonStem,
    CrimsonHyphae,
    WarpedNylium,
    WarpedStem,
    WarpedHyphae,
    WarpedWartBlock,
    Deepslate,
    RawIron,
    GlowLichen,
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
            MapColor::None => return Rgba([0; 4]),
            MapColor::Grass => 8368696_u32,
            MapColor::Sand => 16247203,
            MapColor::Wool => 13092807,
            MapColor::Fire => 16711680,
            MapColor::Ice => 10526975,
            MapColor::Metal => 10987431,
            MapColor::Plant => 31744,
            MapColor::Snow => 16777215,
            MapColor::Clay => 10791096,
            MapColor::Dirt => 9923917,
            MapColor::Stone => 7368816,
            MapColor::Water => 4210943,
            MapColor::Wood => 9402184,
            MapColor::Quartz => 16776437,
            MapColor::ColorOrange => 14188339,
            MapColor::ColorMagenta => 11685080,
            MapColor::ColorLightBlue => 6724056,
            MapColor::ColorYellow => 15066419,
            MapColor::ColorLightGreen => 8375321,
            MapColor::ColorPink => 15892389,
            MapColor::ColorGray => 5000268,
            MapColor::ColorLightGray => 10066329,
            MapColor::ColorCyan => 5013401,
            MapColor::ColorPurple => 8339378,
            MapColor::ColorBlue => 3361970,
            MapColor::ColorBrown => 6704179,
            MapColor::ColorGreen => 6717235,
            MapColor::ColorRed => 10040115,
            MapColor::ColorBlack => 1644825,
            MapColor::Gold => 16445005,
            MapColor::Diamond => 6085589,
            MapColor::Lapis => 4882687,
            MapColor::Emerald => 55610,
            MapColor::Podzol => 8476209,
            MapColor::Nether => 7340544,
            MapColor::TerracottaWhite => 13742497,
            MapColor::TerracottaOrange => 10441252,
            MapColor::TerracottaMagenta => 9787244,
            MapColor::TerracottaLightBlue => 7367818,
            MapColor::TerracottaYellow => 12223780,
            MapColor::TerracottaLightGreen => 6780213,
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
            MapColor::CrimsonNylium => 12398641,
            MapColor::CrimsonStem => 9715553,
            MapColor::CrimsonHyphae => 6035741,
            MapColor::WarpedNylium => 1474182,
            MapColor::WarpedStem => 3837580,
            MapColor::WarpedHyphae => 5647422,
            MapColor::WarpedWartBlock => 1356933,
            MapColor::Deepslate => 6579300,
            MapColor::RawIron => 14200723,
            MapColor::GlowLichen => 8365974,
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
    #[clap(default_value = "out")]
    out_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] Image(#[from] ImageError),
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

impl wheel::CustomExit for Error {
    fn exit(self, cmd_name: &'static str) {
        match self {
            Self::Regions(region_errors) => {
                println!("failed to render {} region{}:", region_errors.len(), if region_errors.len() == 1 { "" } else { "s" });
                for ([x, z], e) in region_errors {
                    println!("{x}, {z}: {e} (debug info: {e:?})");
                }
            }
            _ => {
                eprintln!("{cmd_name}: {self}");
                eprintln!("debug info: {self:?}");
            }
        }
        #[cfg(not(feature = "flamegraph"))] {
            std::process::exit(1)
        }
    }
}

#[wheel::main(max_blocking_threads = 0, custom_exit)]
async fn main(Args { world_dir, out_dir }: Args) -> Result<(), Error> {
    let block_colors = Arc::new(colors::get_block_colors());
    fs::create_dir_all(&out_dir).await?;
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
        let out_dir = &out_dir;
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
                let out_dir = out_dir.clone();
                prev = Some(tokio::task::spawn_blocking(move || {
                    println!("processing region {}, {}", region.coords[0], region.coords[1]);
                    let mut region_img = RgbaImage::new(16 * 32, 16 * 32);
                    for col in &region {
                        let col = match col {
                            Ok(col) => col,
                            Err(e) => {
                                col_errors.lock().insert([x, z], e);
                                return Ok(region)
                            }
                        };
                        let heightmap = col.heightmaps.get("WORLD_SURFACE").unwrap_or(FALLBACK_HEIGHTMAP);
                        for (block_z, row) in heightmap.iter().enumerate() {
                            for (block_x, max_y) in row.iter().enumerate() {
                                let mut col_color = MapColor::None;
                                let mut y = *max_y;
                                while y >= col.y_pos {
                                    let chunk_y = y.div_euclid(16) as i8;
                                    let block_y = y.rem_euclid(16) as usize;
                                    if let Some(chunk) = col.section_at(chunk_y) {
                                        let block = &chunk.block_relative([block_x as u8, block_y as u8, block_z as u8]);
                                        let Some(&color) = block_colors.get(&block.name) else {
                                            y -= 1;
                                            continue
                                        };
                                        col_color = match color {
                                            BlockMapColor::Single(color) => color,
                                            BlockMapColor::Bed { head, foot } => if block.properties.get("part").is_some_and(|part| part == "head") { head } else { foot },
                                            BlockMapColor::Crops { growing, grown } => if block.properties.get("age").is_some_and(|age| age == "7") { grown } else { growing },
                                            BlockMapColor::Pillar { top, side } => if block.properties.get("axis").is_some_and(|axis| axis != "y") { side } else { top },
                                            BlockMapColor::Waterloggable { dry, wet } => if block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true") { wet } else { dry },
                                        };
                                        if col_color != MapColor::None { break }
                                    }
                                    if y == col.y_pos { break }
                                    y -= 1;
                                }
                                let x = col.x_pos * 16 + block_x as i32;
                                let z = col.z_pos * 16 + block_z as i32;
                                let tint = match col_color {
                                    MapColor::None => Tint::Normal,
                                    MapColor::Water => {
                                        let water_depth = (col.y_pos..=y).rev().take_while(|y| {
                                            let chunk_y = y.div_euclid(16) as i8;
                                            let block_y = y.rem_euclid(16) as usize;
                                            if let Some(chunk) = col.section_at(chunk_y) {
                                                let block = &chunk.block_relative([block_x as u8, block_y as u8, block_z as u8]);
                                                let Some(&color) = block_colors.get(&block.name) else { return false };
                                                let col_color = match color {
                                                    BlockMapColor::Single(color) => color,
                                                    BlockMapColor::Bed { head, foot } => if block.properties.get("part").is_some_and(|part| part == "head") { head } else { foot },
                                                    BlockMapColor::Crops { growing, grown } => if block.properties.get("age").is_some_and(|age| age == "7") { grown } else { growing },
                                                    BlockMapColor::Pillar { top, side } => if block.properties.get("axis").is_some_and(|axis| axis != "y") { side } else { top },
                                                    BlockMapColor::Waterloggable { dry, wet } => if block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true") { wet } else { dry },
                                                };
                                                col_color == MapColor::Water || block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true")
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
                                                        let col_color = match color {
                                                            BlockMapColor::Single(color) => color,
                                                            BlockMapColor::Bed { head, foot } => if block.properties.get("part").is_some_and(|part| part == "head") { head } else { foot },
                                                            BlockMapColor::Crops { growing, grown } => if block.properties.get("age").is_some_and(|age| age == "7") { grown } else { growing },
                                                            BlockMapColor::Pillar { top, side } => if block.properties.get("axis").is_some_and(|axis| axis != "y") { side } else { top },
                                                            BlockMapColor::Waterloggable { dry, wet } => if block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true") { wet } else { dry },
                                                        };
                                                        col_color != MapColor::None
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
                                                            let col_color = match color {
                                                                BlockMapColor::Single(color) => color,
                                                                BlockMapColor::Bed { head, foot } => if block.properties.get("part").is_some_and(|part| part == "head") { head } else { foot },
                                                                BlockMapColor::Crops { growing, grown } => if block.properties.get("age").is_some_and(|age| age == "7") { grown } else { growing },
                                                                BlockMapColor::Pillar { top, side } => if block.properties.get("axis").is_some_and(|axis| axis != "y") { side } else { top },
                                                                BlockMapColor::Waterloggable { dry, wet } => if block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true") { wet } else { dry },
                                                            };
                                                            col_color != MapColor::None
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
                    let path = out_dir.join(format!("r.{}.{}.png", region.coords[0], region.coords[1]));
                    let changed = match image::open(&path) { //TODO async
                        Ok(old_img) => RgbaImage::from(old_img) != region_img,
                        Err(ImageError::IoError(e)) if e.kind() == io::ErrorKind::NotFound => true,
                        Err(e) => return Err(e.into()),
                    };
                    if changed {
                        region_img.save_with_format(path, image::ImageFormat::Png)?; //TODO async
                    }
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
        println!("all regions rendered successfully");
        Ok(())
    }
}
