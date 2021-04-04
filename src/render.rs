use lazy_static::lazy_static;
use log::{debug, error};
use sdl2::{
    pixels::PixelFormatEnum,
    rect::Rect,
    render::{BlendMode, Texture, TextureCreator, WindowCanvas},
    video::WindowContext,
};
use std::{borrow::BorrowMut, sync::Mutex};

lazy_static! {
    static ref LAYERS: Mutex<Layers> = Mutex::new(Layers {
        layers: [
            vec![0_u8; LAYERS_SIZE],
            vec![0_u8; LAYERS_SIZE],
            vec![0_u8; LAYERS_SIZE],
            vec![0_u8; LAYERS_SIZE],
            vec![0_u8; LAYERS_SIZE],
            vec![0_u8; LAYERS_SIZE]
        ],
        current_layer: 0,
        allow_pixel_layer: false,
    });
    static ref TILE_MAPS: Mutex<TileMaps> = Mutex::new(TileMaps {
        tile_maps: [
            vec![0_u8; TILE_MAP_SIZE],
            vec![0_u8; TILE_MAP_SIZE],
            vec![0_u8; TILE_MAP_SIZE],
            vec![0_u8; TILE_MAP_SIZE]
        ],
        current_map: 0,
        current_tile: 0,
    });
}

const LAYERS_WIDTH: u32 = 256;
const LAYERS_HEIGHT: u32 = 256;
const LAYERS_PITCH: usize = LAYERS_WIDTH as usize * 4;
const LAYERS_SIZE: usize = LAYERS_PITCH * LAYERS_HEIGHT as usize;
const LAYERS_RECT: (i32, i32, u32, u32) = (0, 0, LAYERS_WIDTH, LAYERS_HEIGHT);
const AMOUNT_LAYERS: usize = 6;
const LAYERS_TYPES: [Layer; AMOUNT_LAYERS] = [
    Layer::Tile,
    Layer::Entity,
    Layer::Tile,
    Layer::Entity,
    Layer::Text,
    Layer::Pixel,
];

#[derive(Eq, PartialEq)]
enum Layer {
    Tile,
    Entity,
    Text,
    Pixel,
}

struct Layers {
    layers: [Vec<u8>; AMOUNT_LAYERS], // Pixel, Text, Entity, Tile, Entity, Tile.
    current_layer: u32,
    allow_pixel_layer: bool,
}

const TILE_WIDTH: u32 = 16;
const TILE_HEIGHT: u32 = 16;
const TILE_PITCH: usize = TILE_WIDTH as usize * 4;
const TILE_SIZE: usize = TILE_PITCH * TILE_HEIGHT as usize;
const AMOUNT_TILES: usize = 32;

const TILE_MAP_SIZE: usize = TILE_SIZE * 32;
const AMOUNT_TILE_MAPS: usize = 4;

struct TileMaps {
    tile_maps: [Vec<u8>; AMOUNT_TILE_MAPS],
    current_map: u32,
    current_tile: u32,
}

pub fn init_textures(tex_creator: &TextureCreator<WindowContext>) -> Result<Vec<Texture>, String> {
    let mut textures = Vec::<Texture>::new();
    for _i in 0..AMOUNT_LAYERS {
        let mut tex = tex_creator
            .create_texture_streaming(PixelFormatEnum::RGBA32, LAYERS_WIDTH, LAYERS_HEIGHT)
            .map_err(|e| e.to_string())?;
        tex.set_blend_mode(BlendMode::Blend);
        textures.push(tex);
    }

    Ok(textures)
}

//TODO Allow tiles to be mapped to the buffer.
pub fn draw(canvas: &mut WindowCanvas, textures: &mut Vec<Texture>) -> Result<(), String> {
    canvas.clear();

    build_textures(textures.borrow_mut())?;
    let (width, height) = canvas.window().size();
    for tex in textures.iter() {
        canvas.copy(&tex, None, Some(Rect::new(0, 0, width, height)))?;
    }

    canvas.present();

    Ok(())
}

// All tile maps are loaded before program start.
// Tiles should be handled in another part of the program.
// pub fn load_tile_maps(maps: &[&[u8; TILE_MAP_SIZE]; AMOUNT_TILE_MAPS]) {}

// This function bypasses the pixel layer check!
pub fn load_image_into_layer(layer: usize, img: &[u8]) {
    if layer < AMOUNT_LAYERS {
        let mut layers = LAYERS.lock().unwrap();
        let mut buffer = layers.layers.get_mut(layer).unwrap();
        buffer.splice(0..LAYERS_SIZE, img.iter().cloned());
    }
    else {
        error!("load_image_into_layer received a layer value greater then the amount of layers!");
    }
}

fn build_textures(textures: &mut Vec<Texture>) -> Result<(), String> {
    if textures.len() == AMOUNT_LAYERS {
        let layers = LAYERS.lock().unwrap();

        for (i, layer) in layers.layers.iter().enumerate() {
            if LAYERS_TYPES[i] != Layer::Pixel
                || (i == AMOUNT_LAYERS - 1 && layers.allow_pixel_layer)
            {
                proc_texture(i, textures.borrow_mut(), layer)?
            }
        }
    }
    else {
        return Err("Wrong number of texture layers were passed to the draw function!".to_string());
    }

    Ok(())
}

fn proc_texture(index: usize, textures: &mut Vec<Texture>, buffer: &[u8]) -> Result<(), String> {
    match textures.get_mut(index) {
        None => return Err("Textures vector is missing a layer!".to_string()),
        Some(tex) => {
            tex.update(Rect::from(LAYERS_RECT), buffer, LAYERS_PITCH as usize)
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub mod colors {
    pub const WHITE: u16 = create_color(15, 15, 15, 15);
    pub const BLACK: u16 = create_color(0, 0, 0, 0);

    pub const fn create_color(r: u8, g: u8, b: u8, a: u8) -> u16 {
        ((((r & 15) as u16) << 12)
            | (((g & 15) as u16) << 8)
            | (((b & 15) as u16) << 4)
            | ((a & 15) as u16))
    }

    pub const fn map_color(color: u16) -> (u8, u8, u8, u8) {
        (
            (15 & ((color >> 12) as u8)) * 17,
            (15 & ((color >> 8) as u8)) * 17,
            (15 & ((color >> 4) as u8)) * 17,
            ((15 & color) as u8) * 17,
        )
    }

    pub fn map_color_vec(color: u16) -> Vec<u8> {
        let (r, g, b, a) = map_color(color);
        vec![r, g, b, a]
    }
}

pub mod commands {
    use crate::render::*;
    use rhai::plugin::*;

    // Color Stuff
    #[export_fn]
    pub fn create_color(r: u8, g: u8, b: u8, a: u8) -> u16 {
        colors::create_color(r, g, b, a)
    }

    // Tile Stuff

    // Pixel Stuff
    #[export_fn]
    pub fn enable_pixel_layer() {
        LAYERS.lock().unwrap().allow_pixel_layer = true;

        debug!("ALLOW PIXEL LAYER");
    }

    #[export_fn]
    pub fn disable_pixel_layer() {
        LAYERS.lock().unwrap().allow_pixel_layer = false;

        debug!("DO NOT ALLOW PIXEL LAYER");
    }

    #[export_fn]
    pub fn draw_pixel(x: u32, y: u32, color: u16) {
        let mut layers = LAYERS.lock().unwrap();
        if !layers.allow_pixel_layer {
            return;
        }

        let buffer = layers.layers.get_mut(AMOUNT_LAYERS - 1).unwrap();

        let offset = (x * 4) as usize + (y as usize * LAYERS_PITCH);
        let rgba: Vec<u8> = colors::map_color_vec(color)
            .iter()
            .map(|val| *val as u8)
            .collect();

        buffer.splice((offset)..(offset + 4), rgba.iter().cloned());
    }
}
