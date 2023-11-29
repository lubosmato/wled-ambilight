mod color_extractor;

use std::ptr::{copy, null};
use std::time::Duration;

use win_desktop_duplication::errors::DDApiError;
use win_desktop_duplication::outputs::{Display, DisplayMode};
use win_desktop_duplication::texture::{ColorFormat, Texture};
use win_desktop_duplication::{
    co_init, set_process_dpi_awareness, DesktopDuplicationApi, DuplicationApiOptions,
};
use win_desktop_duplication::{devices::*, Result};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device4, ID3D11DeviceContext4, ID3D11ShaderResourceView, ID3D11Texture2D,
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_RESOURCE_MISC_GENERATE_MIPS,
    D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
};

use crate::config::Config;

use self::color_extractor::{BorderColors, ColorExtractor, Dimension};

pub struct Screen<'a> {
    config: Config,
    dupl: DesktopDuplicationApi,
    display: Display,
    display_mode: DisplayMode,
    frame_period: Duration,
    scale_factor: u32,
    frame_texture: Option<Texture>,
    mip_texture: Option<ID3D11Texture2D>,
    mip_srv: Option<ID3D11ShaderResourceView>,
    device: ID3D11Device4,
    ctx: ID3D11DeviceContext4,
    color_extractor: Option<ColorExtractor<'a>>,
    frame_data: Vec<u8>,
}

impl<'a> Screen<'a> {
    pub fn new(config: Config) -> Self {
        set_process_dpi_awareness();
        co_init();

        let adapter = AdapterFactory::new()
            .get_adapter_by_idx(config.gpu_index)
            .unwrap();
        let display = adapter.get_display_by_idx(config.display_index).unwrap();

        // TODO solve HDR error
        // TODO sometimes cursor error: Error Unexpected("failed to get DC for cursor image. Error { code: 0x887A0001, message: ...
        let mut dupl = DesktopDuplicationApi::new(adapter, display.clone()).unwrap();
        dupl.configure(DuplicationApiOptions {
            skip_cursor: !config.include_cursor,
        });

        let (device, ctx) = dupl.get_device_and_ctx();

        let mut screen = Self {
            config,
            dupl,
            display,
            display_mode: Default::default(),
            frame_period: Default::default(),
            scale_factor: 0,
            frame_texture: None,
            mip_texture: None,
            mip_srv: None,
            device,
            ctx,
            color_extractor: None,
            frame_data: vec![],
        };

        screen.refresh_display_mode();
        screen
    }

    fn refresh_display_mode(&mut self) {
        self.display_mode = self.display.get_current_display_mode().unwrap();
        self.frame_period = Duration::from_millis((1000.0 / (self.config.max_fps as f32)) as u64);

        let scale_factor_h =
            ((self.display_mode.width as f32) / (self.config.led_horizontal_count as f32)).log2();
        let scale_factor_v =
            ((self.display_mode.height as f32) / (self.config.led_vertical_count as f32)).log2();

        self.scale_factor = scale_factor_h.min(scale_factor_v).abs().floor() as u32;

        let frame_width = self.display_mode.width >> self.scale_factor;
        let frame_height = self.display_mode.height >> self.scale_factor;

        self.color_extractor = Some(ColorExtractor::new(
            Dimension {
                width: frame_width,
                height: frame_height,
            },
            Dimension {
                width: self.config.led_horizontal_count,
                height: self.config.led_vertical_count + 2,
            },
        ));
        self.frame_data
            .reserve((frame_width * frame_height * 4) as usize);

        println!(
            "refreshing display mode: {:?}, scale factor: {}",
            self.display_mode, self.scale_factor
        );
    }

    pub fn wait_for_next_frame(&self) {
        if self.config.enable_v_sync {
            self.display.wait_for_vsync().unwrap();
        } else {
            std::thread::sleep(self.frame_period);
        }
    }

    pub fn get_border_colors(&mut self) -> Option<&BorderColors> {
        return match self.dupl.acquire_next_frame_now() {
            Err(DDApiError::AccessLost) => {
                self.refresh_display_mode();
                None
            }
            Err(err) => {
                println!("Error {:?}", err);
                None
            }
            Ok(tex) => {
                let format = self.get_resized_frame(&tex).unwrap();

                let extractor = self.color_extractor.as_mut().unwrap();
                let colors = extractor.get_border_colors(&mut self.frame_data, format);

                Some(colors)
            }
        };
    }

    fn get_resized_frame(&mut self, input_frame: &Texture) -> Result<ColorFormat> {
        self.resize_into_frame_texture(input_frame)?;

        let raw_tex = self.frame_texture.as_mut().unwrap().as_raw_ref();
        let sub_res = unsafe { self.ctx.Map(raw_tex, 0, D3D11_MAP_READ, 0) };
        if sub_res.is_err() {
            return Err(DDApiError::Unexpected(format!(
                "failed to map to cpu {:?}",
                sub_res
            )));
        }
        let sub_res: D3D11_MAPPED_SUBRESOURCE = sub_res.unwrap();

        let desc = input_frame.desc();
        let width = desc.width >> self.scale_factor;
        let height = desc.height >> self.scale_factor;

        match desc.format {
            ColorFormat::ABGR8UNorm | ColorFormat::ARGB8UNorm | ColorFormat::AYUV => {
                let total_size = width * height * 4;
                self.frame_data.resize(total_size as usize, 0);
                for i in 0..height {
                    unsafe {
                        copy(
                            sub_res.pData.add((i * sub_res.RowPitch) as usize) as *const u8,
                            self.frame_data.as_mut_ptr().add((i * width * 4) as _),
                            (width * 4) as usize,
                        );
                    }
                }
            }
            ColorFormat::YUV444 => {
                let total_size = width * height * 3;
                self.frame_data.resize(total_size as usize, 0);
                for i in 0..(height * 3) {
                    unsafe {
                        copy(
                            sub_res.pData.add((i * sub_res.RowPitch) as usize) as *const u8,
                            self.frame_data.as_mut_ptr().add((i * width) as _),
                            (width) as usize,
                        );
                    }
                }
            }
            ColorFormat::NV12 => {
                let total_size = width * height * 3 / 2;
                self.frame_data.resize(total_size as usize, 0);
                for i in 0..(3 * height / 2) {
                    unsafe {
                        copy(
                            sub_res.pData.add((i * sub_res.RowPitch) as usize) as *const u8,
                            self.frame_data.as_mut_ptr().add((i * width) as _),
                            (width) as usize,
                        );
                    }
                }
            }

            _ => unimplemented!(),
        }
        unsafe {
            self.ctx.Unmap(raw_tex, 0);
        }

        Ok(desc.format)
    }

    fn resize_into_frame_texture(&mut self, input_frame: &Texture) -> Result<()> {
        self.ensure_frame_texture_created(input_frame)?;

        unsafe {
            self.ctx.CopySubresourceRegion(
                self.mip_texture.as_ref().unwrap(),
                0,
                0,
                0,
                0,
                input_frame.as_raw_ref(),
                0,
                null(),
            );

            self.ctx.GenerateMips(self.mip_srv.as_ref().unwrap());

            self.ctx.CopySubresourceRegion(
                self.frame_texture.as_mut().unwrap().as_raw_ref(),
                0,
                0,
                0,
                0,
                self.mip_texture.as_ref().unwrap(),
                self.scale_factor,
                null(),
            );
            self.ctx.Flush();
        }

        Ok(())
    }

    fn ensure_frame_texture_created(&mut self, tex: &Texture) -> Result<()> {
        if self.frame_texture.is_none()
            || self.frame_texture.as_mut().unwrap().desc().height
                != tex.desc().height >> self.scale_factor
            || self.frame_texture.as_mut().unwrap().desc().width
                != tex.desc().width >> self.scale_factor
        {
            self.frame_texture = None;
            let mut desc = Default::default();
            unsafe { tex.as_raw_ref().GetDesc(&mut desc) };
            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = Default::default();
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
            desc.MiscFlags = Default::default();
            desc.Width >>= self.scale_factor;
            desc.Height >>= self.scale_factor;

            let new_tex = unsafe { self.device.CreateTexture2D(&desc, null()) };
            if new_tex.is_err() {
                return Err(DDApiError::Unexpected(format!(
                    "failed to create texture. {:?}",
                    new_tex
                )));
            }

            let mut mip_desc = Default::default();
            unsafe { tex.as_raw_ref().GetDesc(&mut mip_desc) }

            mip_desc.BindFlags = D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET;
            mip_desc.MiscFlags = D3D11_RESOURCE_MISC_GENERATE_MIPS;
            mip_desc.MipLevels = 0;
            mip_desc.Usage = D3D11_USAGE_DEFAULT;
            mip_desc.CPUAccessFlags = Default::default();

            unsafe {
                println!("new mip texture");
                self.mip_texture = Some(self.device.CreateTexture2D(&mip_desc, null()).unwrap());

                self.mip_srv = Some(
                    self.device
                        .CreateShaderResourceView(self.mip_texture.as_ref().unwrap(), null())
                        .unwrap(),
                );
            }

            self.frame_texture = Some(Texture::new(new_tex.unwrap()))
        }

        Ok(())
    }
}
