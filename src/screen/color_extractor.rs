use std::num::NonZeroU32;

use fast_image_resize as fir;
use win_desktop_duplication::texture::ColorFormat;

#[derive(PartialEq, PartialOrd)]
pub struct Dimension {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct BorderColors {
    pub top: Vec<u8>,
    pub right: Vec<u8>,
    pub bottom: Vec<u8>,
    pub left: Vec<u8>,
}

impl BorderColors {
    pub fn concat(&self) -> Vec<u8> {
        let mut all_colors = Vec::new();
        all_colors.reserve(self.top.len() + self.right.len() + self.bottom.len() + self.left.len());
        all_colors.extend_from_slice(self.top.as_slice());
        all_colors.extend_from_slice(self.right.as_slice());
        all_colors.extend_from_slice(self.bottom.as_slice());
        all_colors.extend_from_slice(self.left.as_slice());

        all_colors
    }
}

pub struct ColorExtractor<'a> {
    resizer: fir::Resizer,
    dest_image: fir::Image<'a>,
    source_dim: Dimension,
    colors: BorderColors,
}

impl<'a> ColorExtractor<'a> {
    pub fn new(source_dim: Dimension, dest_dim: Dimension) -> Self {
        let mut colors = BorderColors {
            top: vec![],
            right: vec![],
            bottom: vec![],
            left: vec![],
        };
        colors.top.resize((dest_dim.width * 4) as usize, 0);
        colors.right.resize(((dest_dim.height - 2) * 4) as usize, 0);
        colors.bottom.resize((dest_dim.width * 4) as usize, 0);
        colors.left.resize(((dest_dim.height - 2) * 4) as usize, 0);

        Self {
            resizer: fir::Resizer::new(fir::ResizeAlg::Convolution(fir::FilterType::Bilinear)),
            dest_image: fast_image_resize::Image::new(
                NonZeroU32::new(dest_dim.width).unwrap(),
                NonZeroU32::new(dest_dim.height).unwrap(),
                fast_image_resize::PixelType::U8x4,
            ),
            source_dim,
            colors,
        }
    }

    pub fn get_border_colors(&mut self, pixels: &mut [u8], format: ColorFormat) -> &BorderColors {
        let source_image = fast_image_resize::Image::from_slice_u8(
            NonZeroU32::new(self.source_dim.width).unwrap(),
            NonZeroU32::new(self.source_dim.height).unwrap(),
            pixels,
            fast_image_resize::PixelType::U8x4,
        )
        .unwrap();

        self.resizer
            .resize(&source_image.view(), &mut self.dest_image.view_mut())
            .unwrap();

        let width = self.dest_image.width().get() as usize;
        let height = self.dest_image.height().get() as usize;

        let buffer = self.dest_image.buffer_mut();

        match format {
            ColorFormat::ABGR8UNorm => {
                buffer.iter_mut().skip(3).step_by(4).for_each(|c| *c = 0);
                buffer.chunks_mut(4).for_each(|c| {
                    let r = c[2];
                    c[2] = c[0];
                    c[0] = r;
                });
            }
            _ => unimplemented!("sorry, this format is not implemented"),
        }

        // top horizontal row
        self.colors.top.copy_from_slice(&buffer[0..width * 4]);

        // right vertical row
        self.colors.right.clear();
        for row in 1..height - 1 {
            let buffer_begin = row * width * 4 + (width - 1) * 4;
            self.colors
                .right
                .extend_from_slice(&buffer[buffer_begin..buffer_begin + 4]);
        }

        // bottom horizontal row
        self.colors.bottom.clear();
        for col in (0..width).rev() {
            let buffer_begin = (width * 4 * (height - 1)) + col * 4;
            self.colors
                .bottom
                .extend_from_slice(&buffer[buffer_begin..buffer_begin + 4]);
        }

        // left vertical row
        self.colors.left.clear();
        for row in (1..height - 1).rev() {
            let buffer_begin = row * width * 4;
            self.colors
                .left
                .extend_from_slice(&buffer[buffer_begin..buffer_begin + 4]);
        }

        &self.colors
    }
}
