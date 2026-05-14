// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::os::windows::ffi::OsStrExt;
use std::{ffi::c_void, iter::once, path::Path};
use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::{
    Graphics::{
        Gdi::{CreateBitmap, HBITMAP},
        Imaging::{
            CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICBitmapDecoder,
            IWICImagingFactory, WICBitmapInterpolationModeFant, WICConvertBitmapSource,
            WICDecodeMetadataCacheOnDemand,
        },
    },
    System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
};

use crate::Result;

const DRAG_IMAGE_HEIGHT: u32 = 32;

pub(crate) fn read_bytes_to_hbitmap(bytes: &[u8]) -> Result<HBITMAP> {
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

        let stream = factory.CreateStream()?;
        stream.InitializeFromMemory(bytes)?;

        let decoder = factory.CreateDecoderFromStream(
            &stream,
            std::ptr::null(),
            WICDecodeMetadataCacheOnDemand,
        )?;

        decoder_to_hbitmap(&factory, decoder)
    }
}

pub(crate) fn read_path_to_hbitmap(path: &Path) -> Result<HBITMAP> {
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;

        let path = dunce::canonicalize(path)?;
        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(once(0)).collect();

        let decoder = factory.CreateDecoderFromFilename(
            PCWSTR::from_raw(wide_path.as_ptr()),
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )?;

        decoder_to_hbitmap(&factory, decoder)
    }
}

fn decoder_to_hbitmap(factory: &IWICImagingFactory, decoder: IWICBitmapDecoder) -> Result<HBITMAP> {
    unsafe {
        let frame = decoder.GetFrame(0)?;

        let mut width: u32 = 0;
        let mut height: u32 = 0;
        frame.GetSize(&mut width, &mut height)?;

        let scale = DRAG_IMAGE_HEIGHT as f64 / height as f64;
        let width = ((width as f64 * scale).round() as u32).max(1);
        let height = DRAG_IMAGE_HEIGHT;

        let scaler = factory.CreateBitmapScaler()?;
        scaler.Initialize(&frame, width, height, WICBitmapInterpolationModeFant)?;

        let mut pixel_buf: Vec<u8> = vec![0; (width * height * 4) as usize];
        let pixel_format = scaler.GetPixelFormat()?;
        if pixel_format != GUID_WICPixelFormat32bppPBGRA {
            let bitmap_source = WICConvertBitmapSource(&GUID_WICPixelFormat32bppPBGRA, &scaler)?;
            bitmap_source.CopyPixels(std::ptr::null(), width * 4, &mut pixel_buf)?;
        } else {
            scaler.CopyPixels(std::ptr::null(), width * 4, &mut pixel_buf)?;
        }

        Ok(CreateBitmap(
            width as i32,
            height as i32,
            1,
            32,
            Some(pixel_buf.as_ptr() as *const c_void),
        ))
    }
}
