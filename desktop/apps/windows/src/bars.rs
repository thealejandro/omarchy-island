//! The three equalizer bars, alive while music plays.
//!
//! Each bar is a DirectComposition visual whose height is a sine animation
//! that DWM runs: the island's process stays asleep while they move. Paused,
//! they rest flat.

use island_core::shape::Rect;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    D2D1_ROUNDED_RECT, ID2D1DeviceContext, ID2D1Image,
};
use windows::Win32::Graphics::DirectComposition::{
    IDCompositionDevice, IDCompositionEffectGroup, IDCompositionScaleTransform,
    IDCompositionSurface, IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dxgi::IDXGISurface;
use windows::core::Result;
use windows_numerics::Matrix3x2;

/// In DIPs, as in the layout's bar box.
const WIDTH: f32 = 3.0;
const STEP: f32 = 7.0;
/// Paused: this much of the full height.
const REST: f32 = 0.25;
/// Per bar: middle and swing of its height (fractions of the full one),
/// beats per second and phase in degrees, so they never move together.
const WAVES: [(f32, f32, f32, f32); 3] = [
    (0.55, 0.25, 1.9, 0.0),
    (0.7, 0.3, 1.45, 120.0),
    (0.6, 0.3, 2.3, 240.0),
];

struct Bar {
    visual: IDCompositionVisual,
    scale: IDCompositionScaleTransform,
    effect: IDCompositionEffectGroup,
}

pub struct Bars {
    bars: [Bar; 3],
    /// The bars' picture: size in px and color it was painted with.
    surface: Option<(IDCompositionSurface, u32, u32, [f32; 3])>,
    playing: Option<bool>,
}

impl Bars {
    pub fn new(dcomp: &IDCompositionDevice, root: &IDCompositionVisual) -> Result<Self> {
        let bar = || -> Result<Bar> {
            unsafe {
                let visual = dcomp.CreateVisual()?;
                let scale = dcomp.CreateScaleTransform()?;
                let effect = dcomp.CreateEffectGroup()?;
                visual.SetTransform(&scale)?;
                visual.SetEffect(&effect)?;
                effect.SetOpacity2(0.0)?;
                root.AddVisual(&visual, true, None::<&IDCompositionVisual>)?;
                Ok(Bar {
                    visual,
                    scale,
                    effect,
                })
            }
        };
        Ok(Self {
            bars: [bar()?, bar()?, bar()?],
            surface: None,
            playing: None,
        })
    }

    /// One bar at full height, `w` x `h` px, in `color`; all three show it.
    fn paint(
        &mut self,
        dcomp: &IDCompositionDevice,
        dc: &ID2D1DeviceContext,
        w: u32,
        h: u32,
        color: [f32; 3],
    ) -> Result<()> {
        if self
            .surface
            .as_ref()
            .is_some_and(|s| (s.1, s.2, s.3) == (w, h, color))
        {
            return Ok(());
        }
        unsafe {
            let surface = dcomp.CreateSurface(
                w,
                h,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_ALPHA_MODE_PREMULTIPLIED,
            )?;
            let mut offset = POINT::default();
            let dxgi: IDXGISurface = surface.BeginDraw(None, &mut offset)?;
            let props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                ..Default::default()
            };
            let bitmap = dc.CreateBitmapFromDxgiSurface(&dxgi, Some(&props))?;
            dc.SetTarget(&bitmap);
            dc.BeginDraw();
            let (ox, oy) = (offset.x as f32, offset.y as f32);
            dc.SetTransform(&Matrix3x2::translation(ox, oy));
            dc.PushAxisAlignedClip(
                &D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: w as f32,
                    bottom: h as f32,
                },
                windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_ALIASED,
            );
            dc.Clear(Some(&D2D1_COLOR_F::default()));
            let [r, g, b] = color;
            let brush = dc.CreateSolidColorBrush(&D2D1_COLOR_F { r, g, b, a: 0.9 }, None)?;
            let radius = w as f32 / 2.0;
            dc.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: w as f32,
                        bottom: h as f32,
                    },
                    radiusX: radius,
                    radiusY: radius,
                },
                &brush,
            );
            dc.PopAxisAlignedClip();
            let ended = dc.EndDraw(None, None);
            dc.SetTarget(None::<&ID2D1Image>);
            surface.EndDraw()?;
            ended?;
            for bar in &self.bars {
                bar.visual.SetContent(&surface)?;
                bar.scale.SetCenterY2(h as f32 / 2.0)?;
            }
            self.surface = Some((surface, w, h, color));
        }
        Ok(())
    }

    /// Puts the bars in `rect` (px, in the window) at `alpha`. Only a change
    /// between playing and paused touches the animations.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        dcomp: &IDCompositionDevice,
        dc: &ID2D1DeviceContext,
        rect: Rect,
        scale: f32,
        color: [f32; 3],
        alpha: f32,
        playing: bool,
    ) -> Result<()> {
        let (w, h) = (
            (WIDTH * scale).round().max(1.0),
            rect.height.round().max(1.0),
        );
        self.paint(dcomp, dc, w as u32, h as u32, color)?;
        let top = (rect.y + (rect.height - h) / 2.0).round();
        unsafe {
            for (i, bar) in self.bars.iter().enumerate() {
                let x = (rect.x + i as f32 * STEP * scale).round();
                bar.visual.SetOffsetX2(x)?;
                bar.visual.SetOffsetY2(top)?;
                bar.effect.SetOpacity2(alpha)?;
            }
            if self.playing != Some(playing) {
                self.playing = Some(playing);
                for (bar, (mid, swing, hz, phase)) in self.bars.iter().zip(WAVES) {
                    if playing {
                        let wave = dcomp.CreateAnimation()?;
                        wave.AddSinusoidal(0.0, mid, swing, hz, phase)?;
                        bar.scale.SetScaleY(&wave)?;
                    } else {
                        bar.scale.SetScaleY2(REST)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn hide(&self) -> Result<()> {
        for bar in &self.bars {
            unsafe { bar.effect.SetOpacity2(0.0)? };
        }
        Ok(())
    }
}
