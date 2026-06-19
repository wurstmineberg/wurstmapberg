#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
        borrow::Cow,
        collections::HashMap,
        convert::Infallible as Never,
        fs::File,
        io::prelude::*,
        num::NonZero,
        str::FromStr,
    },
    async_tempfile::TempDir,
    futures::{
        future,
        stream::TryStreamExt as _,
    },
    image::Rgba,
    itertools::Itertools as _,
    mcanvil::RegionDecodeError,
    tokio::process::Command,
    wheel::{
        fs,
        traits::AsyncCommandOutputExt as _,
    },
};

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

impl MapColor {
    fn from_dye(s: &str) -> Result<Self, Error> {
        Ok(match s {
            //TODO generate from Java (net/minecraft/world/item/DyeColor.java)
            "WHITE" => Self::Snow,
            "ORANGE" => Self::ColorOrange,
            "MAGENTA" => Self::ColorMagenta,
            "LIGHT_BLUE" => Self::ColorLightBlue,
            "YELLOW" => Self::ColorYellow,
            "LIME" => Self::ColorLightGreen,
            "PINK" => Self::ColorPink,
            "GRAY" => Self::ColorGray,
            "LIGHT_GRAY" => Self::ColorLightGray,
            "CYAN" => Self::ColorCyan,
            "PURPLE" => Self::ColorPurple,
            "BLUE" => Self::ColorBlue,
            "BROWN" => Self::ColorBrown,
            "GREEN" => Self::ColorGreen,
            "RED" => Self::ColorRed,
            "BLACK" => Self::ColorBlack,
            _ => return Err(Error::UnknownDyeColor(s.to_owned())),
        })
    }
}

impl FromStr for MapColor {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Ok(match s {
            "NONE" => Self::None,
            "GRASS" => Self::Grass,
            "SAND" => Self::Sand,
            "WOOL" => Self::Wool,
            "FIRE" => Self::Fire,
            "ICE" => Self::Ice,
            "METAL" => Self::Metal,
            "PLANT" => Self::Plant,
            "SNOW" => Self::Snow,
            "CLAY" => Self::Clay,
            "DIRT" => Self::Dirt,
            "STONE" => Self::Stone,
            "WATER" => Self::Water,
            "WOOD" => Self::Wood,
            "QUARTZ" => Self::Quartz,
            "COLOR_ORANGE" => Self::ColorOrange,
            "COLOR_MAGENTA" => Self::ColorMagenta,
            "COLOR_LIGHT_BLUE" => Self::ColorLightBlue,
            "COLOR_YELLOW" => Self::ColorYellow,
            "COLOR_LIGHT_GREEN" => Self::ColorLightGreen,
            "COLOR_PINK" => Self::ColorPink,
            "COLOR_GRAY" => Self::ColorGray,
            "COLOR_LIGHT_GRAY" => Self::ColorLightGray,
            "COLOR_CYAN" => Self::ColorCyan,
            "COLOR_PURPLE" => Self::ColorPurple,
            "COLOR_BLUE" => Self::ColorBlue,
            "COLOR_BROWN" => Self::ColorBrown,
            "COLOR_GREEN" => Self::ColorGreen,
            "COLOR_RED" => Self::ColorRed,
            "COLOR_BLACK" => Self::ColorBlack,
            "GOLD" => Self::Gold,
            "DIAMOND" => Self::Diamond,
            "LAPIS" => Self::Lapis,
            "EMERALD" => Self::Emerald,
            "PODZOL" => Self::Podzol,
            "NETHER" => Self::Nether,
            "TERRACOTTA_WHITE" => Self::TerracottaWhite,
            "TERRACOTTA_ORANGE" => Self::TerracottaOrange,
            "TERRACOTTA_MAGENTA" => Self::TerracottaMagenta,
            "TERRACOTTA_LIGHT_BLUE" => Self::TerracottaLightBlue,
            "TERRACOTTA_YELLOW" => Self::TerracottaYellow,
            "TERRACOTTA_LIGHT_GREEN" => Self::TerracottaLightGreen,
            "TERRACOTTA_PINK" => Self::TerracottaPink,
            "TERRACOTTA_GRAY" => Self::TerracottaGray,
            "TERRACOTTA_LIGHT_GRAY" => Self::TerracottaLightGray,
            "TERRACOTTA_CYAN" => Self::TerracottaCyan,
            "TERRACOTTA_PURPLE" => Self::TerracottaPurple,
            "TERRACOTTA_BLUE" => Self::TerracottaBlue,
            "TERRACOTTA_BROWN" => Self::TerracottaBrown,
            "TERRACOTTA_GREEN" => Self::TerracottaGreen,
            "TERRACOTTA_RED" => Self::TerracottaRed,
            "TERRACOTTA_BLACK" => Self::TerracottaBlack,
            "CRIMSON_NYLIUM" => Self::CrimsonNylium,
            "CRIMSON_STEM" => Self::CrimsonStem,
            "CRIMSON_HYPHAE" => Self::CrimsonHyphae,
            "WARPED_NYLIUM" => Self::WarpedNylium,
            "WARPED_STEM" => Self::WarpedStem,
            "WARPED_HYPHAE" => Self::WarpedHyphae,
            "WARPED_WART_BLOCK" => Self::WarpedWartBlock,
            "DEEPSLATE" => Self::Deepslate,
            "RAW_IRON" => Self::RawIron,
            "GLOW_LICHEN" => Self::GlowLichen,
            _ => return Err(Error::UnknownColor(s.to_owned())),
        })
    }
}

impl From<MapColor> for Rgba<u8> {
    fn from(color: MapColor) -> Self {
        let rgb = match color {
            MapColor::None => return Self([0; 4]),
            //TODO generate from Java (net/minecraft/world/level/material/MapColor.java)
            MapColor::Grass => 8368696_u32,
            MapColor::Sand => 16247203_u32,
            MapColor::Wool => 13092807_u32,
            MapColor::Fire => 16711680_u32,
            MapColor::Ice => 10526975_u32,
            MapColor::Metal => 10987431_u32,
            MapColor::Plant => 31744_u32,
            MapColor::Snow => 16777215_u32,
            MapColor::Clay => 10791096_u32,
            MapColor::Dirt => 9923917_u32,
            MapColor::Stone => 7368816_u32,
            MapColor::Water => 4210943_u32,
            MapColor::Wood => 9402184_u32,
            MapColor::Quartz => 16776437_u32,
            MapColor::ColorOrange => 14188339_u32,
            MapColor::ColorMagenta => 11685080_u32,
            MapColor::ColorLightBlue => 6724056_u32,
            MapColor::ColorYellow => 15066419_u32,
            MapColor::ColorLightGreen => 8375321_u32,
            MapColor::ColorPink => 15892389_u32,
            MapColor::ColorGray => 5000268_u32,
            MapColor::ColorLightGray => 10066329_u32,
            MapColor::ColorCyan => 5013401_u32,
            MapColor::ColorPurple => 8339378_u32,
            MapColor::ColorBlue => 3361970_u32,
            MapColor::ColorBrown => 6704179_u32,
            MapColor::ColorGreen => 6717235_u32,
            MapColor::ColorRed => 10040115_u32,
            MapColor::ColorBlack => 1644825_u32,
            MapColor::Gold => 16445005_u32,
            MapColor::Diamond => 6085589_u32,
            MapColor::Lapis => 4882687_u32,
            MapColor::Emerald => 55610_u32,
            MapColor::Podzol => 8476209_u32,
            MapColor::Nether => 7340544_u32,
            MapColor::TerracottaWhite => 13742497_u32,
            MapColor::TerracottaOrange => 10441252_u32,
            MapColor::TerracottaMagenta => 9787244_u32,
            MapColor::TerracottaLightBlue => 7367818_u32,
            MapColor::TerracottaYellow => 12223780_u32,
            MapColor::TerracottaLightGreen => 6780213_u32,
            MapColor::TerracottaPink => 10505550_u32,
            MapColor::TerracottaGray => 3746083_u32,
            MapColor::TerracottaLightGray => 8874850_u32,
            MapColor::TerracottaCyan => 5725276_u32,
            MapColor::TerracottaPurple => 8014168_u32,
            MapColor::TerracottaBlue => 4996700_u32,
            MapColor::TerracottaBrown => 4993571_u32,
            MapColor::TerracottaGreen => 5001770_u32,
            MapColor::TerracottaRed => 9321518_u32,
            MapColor::TerracottaBlack => 2430480_u32,
            MapColor::CrimsonNylium => 12398641_u32,
            MapColor::CrimsonStem => 9715553_u32,
            MapColor::CrimsonHyphae => 6035741_u32,
            MapColor::WarpedNylium => 1474182_u32,
            MapColor::WarpedStem => 3837580_u32,
            MapColor::WarpedHyphae => 5647422_u32,
            MapColor::WarpedWartBlock => 1356933_u32,
            MapColor::Deepslate => 6579300_u32,
            MapColor::RawIron => 14200723_u32,
            MapColor::GlowLichen => 8365974_u32,
        };
        let [_, r, g, b] = rgb.to_be_bytes();
        Self([r, g, b, u8::MAX])
    }
}

#[allow(unused)] // debug implementation is used for this crate's output
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

fn parse_dye_color(source: &[u8], dye_color: &tree_sitter::Node<'_>) -> Result<MapColor, Error> {
    Ok(match dye_color.kind() {
        "field_access" => match dye_color.child_by_field_name("object").ok_or(Error::MissingSyntaxNode)?.utf8_text(source)? {
            "DyeColor" => {
                let field = dye_color.child_by_field_name("field").ok_or(Error::MissingSyntaxNode)?;
                match field.kind() {
                    "identifier" => MapColor::from_dye(field.utf8_text(source)?)?,
                    kind => return Err(Error::NodeKind("DyeColor", kind.to_owned())),
                }
            }
            name => return Err(Error::NodeValue("parse_dye_color", name.to_owned())),
        },
        kind => return Err(Error::NodeKind("parse_dye_color", kind.to_owned())),
    })
}

fn parse_map_color(source: &[u8], defs: &HashMap<String, BlockMapColor>, color: &tree_sitter::Node<'_>) -> Result<MapColor, Error> {
    Ok(match color.kind() {
        "field_access" => match color.child_by_field_name("object").ok_or(Error::MissingSyntaxNode)?.utf8_text(source)? {
            "DyeColor" => {
                let field = color.child_by_field_name("field").ok_or(Error::MissingSyntaxNode)?;
                match field.kind() {
                    "identifier" => MapColor::from_dye(field.utf8_text(source)?)?,
                    kind => return Err(Error::NodeKind("DyeColor", kind.to_owned())),
                }
            }
            "MapColor" => {
                let field = color.child_by_field_name("field").ok_or(Error::MissingSyntaxNode)?;
                match field.kind() {
                    "identifier" => field.utf8_text(source)?.parse()?,
                    kind => return Err(Error::NodeKind("MapColor", kind.to_owned())),
                }
            }
            receiver => return Err(Error::NodeValue("parse_map_color field_access", receiver.to_owned())),
        },
        "method_invocation" => match color.child_by_field_name("name").ok_or(Error::MissingSyntaxNode)?.utf8_text(source)? {
            "defaultMapColor" => {
                let receiver = color.child_by_field_name("object").ok_or(Error::MissingSyntaxNode)?;
                match receiver.kind() {
                    "identifier" => {
                        let name = receiver.utf8_text(source)?;
                        match defs.get(name).ok_or_else(|| Error::Undefined(name.to_owned()))? {
                            BlockMapColor::Single(color) => color.to_owned(),
                            BlockMapColor::Bed { foot, .. } => foot.to_owned(),
                            BlockMapColor::Crops { growing, .. } => growing.to_owned(),
                            BlockMapColor::Pillar { top, .. } => top.to_owned(),
                            BlockMapColor::Waterloggable { dry, .. } => dry.to_owned(),
                        }
                    }
                    kind => return Err(Error::NodeKind("defaultMapColor", kind.to_owned())),
                }
            }
            name => return Err(Error::NodeValue("parse_map_color method_invocation", name.to_owned())),
        },
        kind => return Err(Error::NodeKind("parse_map_color", kind.to_owned())),
    })
}

fn map_color_from_settings(source: &[u8], defs: &HashMap<String, BlockMapColor>, settings: &tree_sitter::Node<'_>) -> Result<BlockMapColor, Error> {
    Ok(match settings.kind() {
        "method_invocation" => match settings.child_by_field_name("name").ok_or(Error::MissingSyntaxNode)?.utf8_text(source)? {
            "of" => match settings.child_by_field_name("object").ok_or(Error::MissingSyntaxNode)?.utf8_text(source)? {
                "BlockBehaviour.Properties" => BlockMapColor::Single(MapColor::None),
                receiver => return Err(Error::NodeValue("`of` receiver", receiver.to_owned())),
            },
            "copyLootTable" if settings.child_by_field_name("object").is_none() => BlockMapColor::Single(MapColor::None),
            "ofLegacyCopy" | "ofFullCopy" => match settings.child_by_field_name("object").ok_or(Error::MissingSyntaxNode)?.utf8_text(source)? {
                "BlockBehaviour.Properties" => {
                    let args = settings.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                    if let [block] = &*args.named_children(&mut args.walk()).collect_vec() {
                        let name = block.utf8_text(source)?;
                        defs.get(name).ok_or_else(|| Error::Undefined(name.to_owned()))?.clone()
                    } else {
                        return Err(Error::ArgCount)
                    }
                }
                receiver => return Err(Error::NodeValue("copy receiver", receiver.to_owned())),
            },
            "wallVariant" if settings.child_by_field_name("object").is_none() => {
                let args = settings.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                if let [block, _] = &*args.named_children(&mut args.walk()).collect_vec() {
                    let name = block.utf8_text(source)?;
                    defs.get(name).ok_or_else(|| Error::Undefined(name.to_owned()))?.clone()
                } else {
                    return Err(Error::ArgCount)
                }
            }
            "buttonProperties" if settings.child_by_field_name("object").is_none() => BlockMapColor::Single(MapColor::None),
            "candleProperties" | "netherStemProperties" | "shulkerBoxProperties" if settings.child_by_field_name("object").is_none() => {
                let args = settings.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                if let [color] = &*args.named_children(&mut args.walk()).collect_vec() {
                    BlockMapColor::Single(parse_map_color(source, defs, color)?)
                } else {
                    return Err(Error::ArgCount)
                }
            }
            "flowerPotProperties" if settings.child_by_field_name("object").is_none() => BlockMapColor::Single(MapColor::None),
            "leavesProperties" if settings.child_by_field_name("object").is_none() => BlockMapColor::Single(MapColor::Plant),
            "logProperties" if settings.child_by_field_name("object").is_none() => {
                let args = settings.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                if let [top, side, _] = &*args.named_children(&mut args.walk()).collect_vec() {
                    BlockMapColor::Pillar {
                        top: parse_map_color(source, defs, top)?,
                        side: parse_map_color(source, defs, side)?,
                    }
                } else {
                    return Err(Error::ArgCount)
                }
            }
            "pistonProperties" if settings.child_by_field_name("object").is_none() => BlockMapColor::Single(MapColor::Stone),
            "mapColor" => {
                let args = settings.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                if let [color] = &*args.named_children(&mut args.walk()).collect_vec() {
                    match color.utf8_text(source)? {
                        "waterloggedMapColor(MapColor.NONE)" => BlockMapColor::Waterloggable {
                            dry: MapColor::None,
                            wet: MapColor::Water,
                        },
                        "statex -> statex.getValue(CropBlock.AGE) >= 6 ? MapColor.COLOR_YELLOW : MapColor.PLANT" => BlockMapColor::Crops {
                            growing: MapColor::Plant,
                            grown: MapColor::ColorYellow,
                        },
                        _ => BlockMapColor::Single(parse_map_color(source, defs, color)?),
                    }
                } else {
                    return Err(Error::ArgCount)
                }
            }
            method_name => if let Some(receiver) = settings.child_by_field_name("object") {
                map_color_from_settings(source, defs, &receiver)?
            } else {
                return Err(Error::NodeValue("uncategorized method receiver", method_name.to_owned()))
            },
        },
        kind => return Err(Error::NodeKind("map_color_from_settings", kind.to_owned())),
    })
}

async fn get_block_colors() -> Result<HashMap<String, BlockMapColor>, Error> {
    let tempdir = TempDir::new().await?;
    let fabric_template = tempdir.join("fabric-template");
    gix::prepare_clone("https://github.com/FabricMC/fabric-example-mod.git", &fabric_template)?
        .with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(NonZero::<u32>::MIN))
        .with_ref_name(Some(env!("CARGO_PKG_VERSION")))?
        .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?.0
        .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)?;
    Command::new(fabric_template.join("gradlew")).arg("genSources").current_dir(&fabric_template).check("gradlew").await?;
    let sources = fs::read_dir(fabric_template.join(".gradle").join("loom-cache").join("minecraftMaven").join("net").join("minecraft"))
        .try_filter(|entry| future::ready(entry.file_name().to_str().is_some_and(|filename| filename.starts_with("minecraft-common-"))))
        .try_collect::<Vec<_>>().await?
        .into_iter()
        .exactly_one()?;
    let sources = fs::read_dir(sources.path()).try_filter(|entry| future::ready(entry.file_name().to_str().is_some_and(|filename| filename.starts_with(env!("CARGO_PKG_VERSION"))))).try_collect::<Vec<_>>().await?.into_iter().exactly_one()?;
    let sources = fs::read_dir(sources.path()).try_filter(|entry| future::ready(entry.file_name().to_str().is_some_and(|filename| filename.ends_with("-sources.jar")))).try_collect::<Vec<_>>().await?.into_iter().exactly_one()?;
    let zip_file = async_zip::tokio::read::fs::ZipFileReader::new(sources.path()).await?;
    let sources_dir = tempdir.join("sources");
    let entries = zip_file.file().entries().iter().enumerate().map(|(idx, entry)| Ok((idx, entry.filename().as_str()?.ends_with('/'), sources_dir.join(entry.filename().as_str()?)))).try_collect::<_, Vec<_>, Error>()?;
    for (idx, is_dir, path) in entries {
        if is_dir {
            fs::create_dir_all(path).await?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }
            let mut buf = Vec::default();
            zip_file.reader_with_entry(idx).await?.read_to_end_checked(&mut buf).await?;
            fs::write(path, &buf).await?;
        }
    }
    let mut block_colors = HashMap::new();
    let mut defs = HashMap::default();
    let mut parser = tree_sitter::Parser::default();
    parser.set_language(&tree_sitter_java::LANGUAGE.into())?;
    //Command::new("mv").arg(&sources_dir).arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("sources")).check("mv").await?; //DEBUG
    let blocks_source = fs::read(sources_dir.join("net").join("minecraft").join("world").join("level").join("block").join("Blocks.java")).await?;
    let blocks = parser.parse(&blocks_source, None).expect("language set above");
    let blocks = blocks.root_node();
    for node in blocks.named_children(&mut blocks.walk()) {
        match node.kind() {
            | "block_comment"
            | "import_declaration"
            | "package_declaration"
                => {}
            "class_declaration" => for node in node.named_children(&mut node.walk()) {
                match node.kind() {
                    | "modifiers"
                    | "identifier"
                        => {}
                    "class_body" => {
                        for node in node.named_children(&mut node.walk()) {
                            match node.kind() {
                                | "block_comment"
                                | "method_declaration"
                                | "static_initializer"
                                    => {}
                                "field_declaration" => match node.child_by_field_name("type").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)? {
                                    | "BlockBehaviour.StatePredicate"
                                        => {}
                                    "Block" => {
                                        let decl = node.child_by_field_name("declarator").ok_or(Error::MissingSyntaxNode)?;
                                        let value = decl.child_by_field_name("value").ok_or(Error::MissingSyntaxNode)?;
                                        match value.kind() {
                                            "method_invocation" => match value.child_by_field_name("name").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)? {
                                                "register" if value.child_by_field_name("object").is_none() => {
                                                    let args = value.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                                                    if let [id, settings] | [id, _, settings] = &*args.named_children(&mut args.walk()).collect_vec() {
                                                        let id = match id.kind() {
                                                            "field_access" => match id.child_by_field_name("object").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)? {
                                                                "BlockIds" => Cow::Owned(id.child_by_field_name("field").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?.to_ascii_lowercase()),
                                                                name => return Err(Error::NodeValue("register ID", name.to_owned())),
                                                            },
                                                            "string_literal" => Cow::Borrowed(id.named_children(&mut id.walk()).find(|node| node.kind() == "string_fragment").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?),
                                                            kind => return Err(Error::NodeKind("register", kind.to_owned())),
                                                        };
                                                        let color = map_color_from_settings(&blocks_source, &defs, settings)?;
                                                        block_colors.insert(id, color.clone());
                                                        defs.insert(decl.child_by_field_name("name").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?.to_owned(), color);
                                                    } else {
                                                        return Err(Error::ArgCount)
                                                    }
                                                }
                                                "registerBed" if value.child_by_field_name("object").is_none() => {
                                                    let args = value.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                                                    if let [id, dye_color] = &*args.named_children(&mut args.walk()).collect_vec() {
                                                        match id.kind() {
                                                            "string_literal" => {
                                                                let id = Cow::Borrowed(id.named_children(&mut id.walk()).find(|node| node.kind() == "string_fragment").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?);
                                                                let color = BlockMapColor::Bed {
                                                                    head: MapColor::Wool,
                                                                    foot: parse_dye_color(&blocks_source, dye_color)?,
                                                                };
                                                                block_colors.insert(id, color.clone());
                                                                defs.insert(decl.child_by_field_name("name").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?.to_owned(), color);
                                                            }
                                                            kind => return Err(Error::NodeKind("registerBedBlock", kind.to_owned())),
                                                        }
                                                    } else {
                                                        return Err(Error::ArgCount)
                                                    }
                                                }
                                                "registerLegacyStair" | "registerStair" if value.child_by_field_name("object").is_none() => {
                                                    let args = value.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                                                    if let [id, base_block] = &*args.named_children(&mut args.walk()).collect_vec() {
                                                        match id.kind() {
                                                            "string_literal" => {
                                                                let id = Cow::Borrowed(id.named_children(&mut id.walk()).find(|node| node.kind() == "string_fragment").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?);
                                                                let name = base_block.utf8_text(&blocks_source)?;
                                                                let color = defs.get(name).ok_or_else(|| Error::Undefined(name.to_owned()))?.clone();
                                                                block_colors.insert(id, color.clone());
                                                                defs.insert(decl.child_by_field_name("name").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?.to_owned(), color);
                                                            }
                                                            kind => return Err(Error::NodeKind("registerStair", kind.to_owned())),
                                                        }
                                                    } else {
                                                        return Err(Error::ArgCount)
                                                    }
                                                }
                                                "registerStainedGlass" if value.child_by_field_name("object").is_none() => {
                                                    let args = value.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                                                    if let [id, dye_color] = &*args.named_children(&mut args.walk()).collect_vec() {
                                                        match id.kind() {
                                                            "string_literal" => {
                                                                let id = Cow::Borrowed(id.named_children(&mut id.walk()).find(|node| node.kind() == "string_fragment").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?);
                                                                let color = BlockMapColor::Single(parse_dye_color(&blocks_source, dye_color)?);
                                                                block_colors.insert(id, color.clone());
                                                                defs.insert(decl.child_by_field_name("name").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?.to_owned(), color);
                                                            }
                                                            kind => return Err(Error::NodeKind("registerStainedGlass", kind.to_owned())),
                                                        }
                                                    } else {
                                                        return Err(Error::ArgCount)
                                                    }
                                                }
                                                name => return Err(Error::NodeValue("Block method invocation", name.to_owned())),
                                            }
                                            kind => return Err(Error::NodeKind("Block field", kind.to_owned())),
                                        }
                                    }
                                    "WeatheringCopperBlocks" => {
                                        let decl = node.child_by_field_name("declarator").ok_or(Error::MissingSyntaxNode)?;
                                        let value = decl.child_by_field_name("value").ok_or(Error::MissingSyntaxNode)?;
                                        match value.kind() {
                                            "method_invocation" => match value.child_by_field_name("name").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)? {
                                                "create" if value.child_by_field_name("object").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)? == "WeatheringCopperBlocks" => {
                                                    let args = value.child_by_field_name("arguments").ok_or(Error::MissingSyntaxNode)?;
                                                    if let [id, _, _, _, settings] = &*args.named_children(&mut args.walk()).collect_vec() {
                                                        let id = match id.kind() {
                                                            "string_literal" => Cow::Borrowed(id.named_children(&mut id.walk()).find(|node| node.kind() == "string_fragment").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?),
                                                            kind => return Err(Error::NodeKind("WeatheringCopperBlocks.create id", kind.to_owned())),
                                                        };
                                                        let settings = match settings.kind() {
                                                            "lambda_expression" => settings.child_by_field_name("body").ok_or(Error::MissingSyntaxNode)?,
                                                            kind => return Err(Error::NodeKind("WeatheringCopperBlocks.create settings", kind.to_owned())),
                                                        };
                                                        let color = map_color_from_settings(&blocks_source, &defs, &settings)?;
                                                        block_colors.insert(id, color.clone());
                                                        defs.insert(decl.child_by_field_name("name").ok_or(Error::MissingSyntaxNode)?.utf8_text(&blocks_source)?.to_owned(), color);
                                                    } else {
                                                        return Err(Error::ArgCount)
                                                    }
                                                }
                                                name => return Err(Error::NodeValue("WeatheringCopperBlocks method invocation", name.to_owned())),
                                            }
                                            kind => return Err(Error::NodeKind("Block field", kind.to_owned())),
                                        }
                                    }
                                    field_type => return Err(Error::NodeValue("field_declaration type", field_type.to_owned())),
                                },
                                kind => return Err(Error::NodeKind("class body", kind.to_owned())),
                            }
                        }
                    }
                    kind => return Err(Error::NodeKind("class declaration", kind.to_owned())),
                }
            },
            kind => return Err(Error::NodeKind("root children", kind.to_owned())),
        }
    }
    Ok(block_colors.into_iter().map(|(id, color)| (id.into_owned(), color)).collect())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] AsyncTempFile(#[from] async_tempfile::Error),
    #[error(transparent)] ChunkColumnDecode(#[from] mcanvil::ChunkColumnDecodeError),
    #[error(transparent)] GitCheckout(#[from] gix::clone::checkout::main_worktree::Error),
    #[error(transparent)] GitClone(#[from] gix::clone::Error),
    #[error(transparent)] GitFetch(#[from] gix::clone::fetch::Error),
    #[error(transparent)] GitValidateRefName(#[from] gix::validate::reference::name::Error),
    #[error(transparent)] Image(#[from] image::ImageError),
    #[error(transparent)] Io(#[from] std::io::Error),
    #[error(transparent)] ParseInt(#[from] std::num::ParseIntError),
    #[error(transparent)] RegionDecode(#[from] RegionDecodeError),
    #[error(transparent)] TreeSitter(#[from] tree_sitter::LanguageError),
    #[error(transparent)] Utf8(#[from] std::str::Utf8Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error(transparent)] Zip(#[from] async_zip::error::ZipError),
    #[error("unexpected number of Java method arguments")]
    ArgCount,
    #[error("expected exactly one element")]
    ExactlyOne,
    #[error("incomplete Java syntax tree")]
    MissingSyntaxNode,
    #[error("unexpected node kind at {0} in Java syntax tree: {1}")]
    NodeKind(&'static str, String),
    #[error("unexpected node text at {0} in Java syntax tree: {1}")]
    NodeValue(&'static str, String),
    #[error("reference to undefined block {0}")]
    Undefined(String),
    #[error("unknown map color: {0}")]
    UnknownColor(String),
    #[error("unknown dye color: {0}")]
    UnknownDyeColor(String),
}

impl From<Never> for Error {
    fn from(never: Never) -> Self {
        match never {}
    }
}

impl<I: Iterator> From<itertools::ExactlyOneError<I>> for Error {
    fn from(_: itertools::ExactlyOneError<I>) -> Self {
        Self::ExactlyOne
    }
}

#[wheel::main]
async fn main() -> Result<(), Error> {
    let block_colors = get_block_colors().await?;
    let mut f = File::create("crate/wurstmapberg-cli/src/colors.rs")?;
    writeln!(&mut f, "use {{")?;
    writeln!(&mut f, "    std::collections::HashMap,")?;
    writeln!(&mut f, "    collect_mac::collect,")?;
    writeln!(&mut f, "    mcanvil::BlockId,")?;
    writeln!(&mut f, "    crate::{{")?;
    writeln!(&mut f, "        BlockMapColor::*,")?;
    writeln!(&mut f, "        MapColor::*,")?;
    writeln!(&mut f, "    }},")?;
    writeln!(&mut f, "}};")?;
    writeln!(&mut f)?;
    writeln!(&mut f, concat!("/// Up to date as of Minecraft ", env!("CARGO_PKG_VERSION")))?;
    writeln!(&mut f, "pub(crate) fn get_block_colors() -> HashMap<BlockId, crate::BlockMapColor> {{")?;
    writeln!(&mut f, "    collect![")?;
    for (id, color) in block_colors.into_iter().sorted_by(|(id1, _), (id2, _)| id1.cmp(id2)) {
        writeln!(&mut f, "        BlockId::{} => {color:?},", match format!("minecraft:{id}").parse::<mcanvil::BlockId>()? {
            mcanvil::BlockId::Other(id) => format!("Other({id:?}.to_owned())"),
            id => format!("{id:?}"),
        })?;
    }
    writeln!(&mut f, "    ]")?;
    writeln!(&mut f, "}}")?;
    Ok(())
}
