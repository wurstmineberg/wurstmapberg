#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
        path::{
            Path,
            PathBuf,
        },
        sync::{
            Arc,
            Mutex,
        },
    },
    chrono::prelude::*,
    image::{
        Rgba,
        RgbaImage,
    },
    mcanvil::{
        Dimension,
        Region,
        RegionDecodeError,
    },
    rayon::prelude::*,
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

impl From<MapColor> for Rgba<u8> {
    fn from(color: MapColor) -> Self {
        let rgb = match color {
            MapColor::Clear => return Self([0; 4]),
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
        let [_, r, g, b] = rgb.to_be_bytes();
        Self([r, g, b, u8::MAX])
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

#[derive(Default)]
struct MapImage {
    img: RgbaImage,
    bounds: Option<[i32; 4]>,
}

impl MapImage {
    fn insert(&mut self, [x, z]: [i32; 2], color: MapColor) {
        let [min_x, min_z, max_x, max_z] = self.bounds.get_or_insert_with(|| [x, z, x, z]);
        if x < *min_x || z < *min_z || x >= *max_x || z >= *max_z {
            let old_min_x = *min_x;
            let old_min_z = *min_z;
            *min_x = x.min(*min_x);
            *min_z = z.min(*min_z);
            *max_x = (x + 1).max(*max_x);
            *max_z = (z + 1).max(*max_z);
            self.img = RgbaImage::from_par_fn((*max_x - *min_x).try_into().unwrap(), (*max_z - *min_z).try_into().unwrap(), |ix, iz| {
                self.img.get_pixel_checked(ix + (old_min_x - *min_x) as u32, iz + (old_min_z - *min_z) as u32).copied().unwrap_or_else(|| Rgba([0; 4]))
            });
        }
        self.img[((x - *min_x) as u32, (z - *min_z) as u32)] = color.into();
    }
}

#[derive(clap::Parser)]
#[clap(version)]
struct Args {
    world_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] ChunkColumnDecode(#[from] mcanvil::ChunkColumnDecodeError),
    #[error(transparent)] Image(#[from] image::ImageError),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("{}", .0[0])]
    Regions(Vec<RegionDecodeError>),
}

#[wheel::main]
async fn main(Args { world_dir }: Args) -> Result<(), Error> {
    let block_colors = colors::get_block_colors();
    fs::create_dir_all("out").await?;
    let region_errors = Arc::<Mutex<Vec<_>>>::default();
    Region::all(world_dir, DIMENSION)
        .par_bridge()
        .try_for_each(|region| {
            let region = match region {
                Ok(region) => region,
                Err(e) => {
                    region_errors.lock().unwrap().push(e);
                    return Ok::<_, Error>(())
                }
            };
            println!("{} processing region {}, {}", Local::now().format("%F %T"), region.coords[0], region.coords[1]);
            let mut region_img = MapImage {
                img: RgbaImage::new(16 * 32, 16 * 32),
                bounds: Some([region.coords[0] * 16 * 32, region.coords[1] * 16 * 32, (region.coords[0] + 1) * 16 * 32, (region.coords[0] + 1) * 16 * 32]),
            };
            for col in &region {
                let col = col?;
                for (block_z, row) in col.heightmaps.get("WORLD_SURFACE").unwrap_or_else(|| &FALLBACK_HEIGHTMAP).iter().enumerate() {
                    for (block_x, max_y) in row.iter().enumerate() {
                        let mut col_color = MapColor::Clear;
                        for y in (col.y_pos..=*max_y).rev() {
                            let chunk_y = y.div_euclid(16) as i8;
                            let block_y = y.rem_euclid(16) as usize;
                            if let Some(chunk) = col.section_at(chunk_y) {
                                let block = &chunk.block_relative([block_x as u8, block_y as u8, block_z as u8]);
                                let name = block.name.strip_prefix("minecraft:").unwrap_or(&block.name);
                                let Some(&color) = block_colors.get(name) else { continue };
                                col_color = match color {
                                    BlockMapColor::Single(color) => color,
                                    BlockMapColor::Bed { head, foot } => if block.properties.get("part").is_some_and(|part| part == "head") { head } else { foot },
                                    BlockMapColor::Crops { growing, grown } => if block.properties.get("age").is_some_and(|age| age == "7") { grown } else { growing },
                                    BlockMapColor::Pillar { top, side } => if block.properties.get("axis").is_some_and(|axis| axis != "y") { side } else { top },
                                    BlockMapColor::Waterloggable { dry, wet } => if block.properties.get("waterlogged").is_some_and(|waterlogged| waterlogged == "true") { wet } else { dry },
                                };
                                if col_color != MapColor::Clear { break }
                            }
                        }
                        //TODO special case for water
                        //TODO shading based on heightmap difference
                        let x = col.x_pos * 16 + block_x as i32;
                        let z = col.z_pos * 16 + block_z as i32;
                        region_img.insert([x, z], col_color);
                    }
                }
            }
            region_img.img.save_with_format(Path::new("out").join(format!("r.{}.{}.png", region.coords[0], region.coords[1])), image::ImageFormat::Png)?;
            Ok(())
        })?;
    let region_errors = Arc::into_inner(region_errors).unwrap().into_inner().unwrap();
    if region_errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Regions(region_errors))
    }
}
