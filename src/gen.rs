use noise::{Fbm, MultiFractal, NoiseFn, OpenSimplex};
use rayon::prelude::*;

const GRASS_GREEN: [u8; 3] = [98_u8, 125_u8, 75_u8];
const FOREST_GREEN: [u8; 3] = [34_u8, 139_u8, 34_u8];
const SAND: [u8; 3] = [194_u8, 178_u8, 128_u8];
const GREY: [u8; 3] = [127_u8, 131_u8, 134_u8];
const DARK_GREY: [u8; 3] = [169_u8, 169_u8, 169_u8];
const WHITE: [u8; 3] = [255_u8, 255_u8, 255_u8];
const NAVY_BLUE: [u8; 3] = [0_u8, 0_u8, 128_u8];
const LIGHT_BLUE: [u8; 3] = [173_u8, 216_u8, 230_u8];

// try https://www.youtube.com/watch?v=gsJHzBTPG0Y
fn circle_gradient_erode(bytes: &mut [f64], width: usize, height: usize) {
    let center_x = width / 2 - 1; //find center of the array
    let center_y = height / 2 - 1; //find center of the array
    bytes.par_iter_mut().enumerate().for_each(|(idx, value)| {
        let x = idx % width as usize;
        let y = (idx as f64 / height as f64).floor() as usize;
        let distance_x = (center_x - x) * (center_x - x);
        let distance_y = (center_y - y) * (center_y - y);

        //find distance from center
        let mut distance_to_center = (distance_x as f64 + distance_y as f64).sqrt();
        distance_to_center = distance_to_center / height as f64;

        *value -= distance_to_center; //set value
    })
}

fn square_gradient_erode(bytes: &mut [f64], width: usize, height: usize) {
    let falloff = 0.5;
    bytes.par_iter_mut().enumerate().for_each(|(idx, value)| {
        let x = (idx % width as usize) as f64;
        let y = (idx as f64 / height as f64).floor() as f64;
        let x_value = (x * 2. - width as f64).abs() / width as f64;
        let y_value = (y * 2. - height as f64).abs() / height as f64;
        let distance_to_center = (x_value.max(y_value) - falloff).clamp(0., 1.) * 2_f64;
        *value -= distance_to_center;
    })
}

pub struct Gen {
    pub height: u32,
    pub width: u32,
    pub octaves: usize,
    pub frequency: f64,
    pub lacunarity: f64,
    pub persistence: f64,
    pub seed: u32,
    x_bounds: (f64, f64),
    y_bounds: (f64, f64),
}

impl Default for Gen {
    fn default() -> Self {
        Self::new()
    }
}

impl Gen {
    pub fn new() -> Self {
        Self {
            octaves: 11,
            height: 1024,
            width: 1024,
            seed: 0,
            frequency: 0.3,
            lacunarity: 2.5,
            persistence: 0.6,
            x_bounds: (-5.0, 10.0),
            y_bounds: (-5.0, 10.0),
        }
    }
}

impl Gen {
    pub fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
    }

    pub fn set_frequency(&mut self, frequency: f64) {
        self.frequency = frequency;
    }

    pub fn set_lacunarity(&mut self, lacunarity: f64) {
        self.lacunarity = lacunarity;
    }

    pub fn set_persistence(&mut self, persistence: f64) {
        self.persistence = persistence;
    }

    pub fn set_octaves(&mut self, octaves: usize) {
        self.octaves = octaves;
    }

    // Statically sized and statically allocated 2x3 matrix using 32-bit floats.
    pub fn gen(&self, buf: &mut [u8]) {
        //
        // calculate
        // should be between -1,1
        // so anything below 0 is water
        let simpler_noise_map = Fbm::<OpenSimplex>::new(self.seed)
            .set_octaves(self.octaves)
            .set_frequency(self.frequency)
            .set_lacunarity(self.lacunarity)
            .set_persistence(self.persistence);
        let x_bounds = self.x_bounds;
        let y_bounds = self.y_bounds;
        let x_extent = x_bounds.1 - x_bounds.0;
        let y_extent = y_bounds.1 - y_bounds.0;

        let height = self.height;
        let width = self.width;

        let x_step = x_extent / width as f64;
        let y_step = y_extent / height as f64;
        // todo: This can probably be moved into a struct with helper functions.
        // It's essetially a 2d matrix backed by a single vec.
        // We have to store on the heap as theres not enough stack space unfortunately.
        // todo: For cliffs we can have another simplex noise, we traverse where land meet's water and
        // sample the noise.
        let mut height_map = vec![0_f64; width as usize * height as usize];
        // todo: The parralism is a bit OTT, but keep's debug runs fast.
        height_map
            .par_iter_mut()
            .enumerate()
            .for_each(|(idx, value)| {
                let x = idx % width as usize;
                let y = (idx as f64 / width as f64).floor() as usize;
                let current_y = y_bounds.0 + y_step * y as f64;
                let current_x = x_bounds.0 + x_step * x as f64;
                let height = simpler_noise_map.get([current_x, current_y]);
                *value = height;
            });

        square_gradient_erode(&mut height_map, width as usize, height as usize);

        // todo: implement these as arrays and simdeez nuts subtract?
        // let square_gradient = [0_f64; WIDTH as usize * HEIGHT as usize];
        // let island_height_map: [f64; WIDTH as usize * HEIGHT as usize] =
        //     array::from_fn(|i| (height_map[i] - square_gradient[i]).min(0.));

        let mut outcome = vec![[0, 0, 0]; width as usize * height as usize];
        for (idx, height) in height_map.iter().enumerate() {
            outcome[idx] = if height > &0.7 {
                WHITE
            } else if height <= &0.7 && height > &0.6 {
                DARK_GREY
            } else if height <= &0.6 && height > &0.5 {
                GREY
            } else if height <= &0.5 && height > &0.25 {
                FOREST_GREEN
            } else if height <= &0.25 && height > &0.0 {
                GRASS_GREEN
            } else if height <= &0.0 && height > &-0.05 {
                SAND
            } else if height >= &-0.15 {
                LIGHT_BLUE
            } else {
                NAVY_BLUE
            }
        }
        let mut encoder = png::Encoder::new(buf, width, height); // Width is 2 pixels and height is 1.
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&outcome.flatten()).unwrap(); // Save
    }
}
