//! Drawing: Direct2D into a DirectComposition surface, so the window has no
//! redirection bitmap and is transparent wherever the island isn't drawn.

use island_core::shape::{Outline, Point, Rect, Segment};
use windows::Win32::Foundation::{E_FAIL, HMODULE, HWND, POINT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
    D2D1_FIGURE_END_CLOSED, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_ALIASED, D2D1_ARC_SEGMENT, D2D1_ARC_SIZE_SMALL,
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, D2D1CreateFactory,
    ID2D1Brush, ID2D1DeviceContext, ID2D1Factory1, ID2D1Image, ID2D1PathGeometry,
    ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionSurface, IDCompositionTarget,
    IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dxgi::{IDXGIAdapter, IDXGIDevice, IDXGISurface};
use windows::core::{Error, Interface, Result};
use windows_numerics::{Matrix3x2, Vector2};

pub struct Renderer {
    factory: ID2D1Factory1,
    dc: ID2D1DeviceContext,
    dcomp: IDCompositionDevice,
    // Kept alive: dropping the target detaches the visual from the window.
    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
    surface: Option<(IDCompositionSurface, u32, u32)>,
    fill: ID2D1SolidColorBrush,
}

fn d3d_device() -> Result<ID3D11Device> {
    // WARP (software) first: the island is small, and the GPU driver alone
    // costs ~50 MB of private memory (7 MB with WARP, measured on an Intel
    // UHD 730). The GPU only if WARP fails.
    for driver in [D3D_DRIVER_TYPE_WARP, D3D_DRIVER_TYPE_HARDWARE] {
        let mut device = None;
        let made = unsafe {
            D3D11CreateDevice(
                None::<&IDXGIAdapter>,
                driver,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        };
        if let (Ok(()), Some(device)) = (made, device) {
            return Ok(device);
        }
    }
    Err(Error::from(E_FAIL))
}

fn v(p: Point) -> Vector2 {
    Vector2 { X: p.x, Y: p.y }
}

impl Renderer {
    pub fn new(hwnd: HWND) -> Result<Self> {
        let dxgi: IDXGIDevice = d3d_device()?.cast()?;
        unsafe {
            let factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dc = factory
                .CreateDevice(&dxgi)?
                .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
            let target = dcomp.CreateTargetForHwnd(hwnd, true)?;
            let visual = dcomp.CreateVisual()?;
            target.SetRoot(&visual)?;
            let fill = dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
                None,
            )?;
            Ok(Self {
                factory,
                dc,
                dcomp,
                _target: target,
                visual,
                surface: None,
                fill,
            })
        }
    }

    fn surface(&mut self, width: u32, height: u32) -> Result<IDCompositionSurface> {
        if let Some((s, w, h)) = &self.surface
            && (*w, *h) == (width, height)
        {
            return Ok(s.clone());
        }
        let s = unsafe {
            let s = self.dcomp.CreateSurface(
                width,
                height,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_ALPHA_MODE_PREMULTIPLIED,
            )?;
            self.visual.SetContent(&s)?;
            s
        };
        self.surface = Some((s.clone(), width, height));
        Ok(s)
    }

    fn geometry(&self, o: &Outline) -> Result<ID2D1PathGeometry> {
        unsafe {
            let geo: ID2D1PathGeometry = self.factory.CreatePathGeometry()?.cast()?;
            let sink = geo.Open()?;
            sink.BeginFigure(v(o.start), D2D1_FIGURE_BEGIN_FILLED);
            for s in &o.segments {
                match *s {
                    Segment::Line(p) => sink.AddLine(v(p)),
                    Segment::Arc {
                        to,
                        radius,
                        clockwise,
                    } => sink.AddArc(&D2D1_ARC_SEGMENT {
                        point: v(to),
                        size: D2D_SIZE_F {
                            width: radius,
                            height: radius,
                        },
                        rotationAngle: 0.0,
                        sweepDirection: if clockwise {
                            D2D1_SWEEP_DIRECTION_CLOCKWISE
                        } else {
                            D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE
                        },
                        arcSize: D2D1_ARC_SIZE_SMALL,
                    }),
                }
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;
            Ok(geo)
        }
    }

    /// Redraws the whole window (`width` x `height` px): the island's
    /// `outline`, placed at `body`, on a transparent background.
    pub fn draw(&mut self, width: u32, height: u32, body: Rect, outline: &Outline) -> Result<()> {
        let geo = self.geometry(outline)?;
        let surface = self.surface(width.max(1), height.max(1))?;
        unsafe {
            let mut offset = POINT::default();
            let dxgi: IDXGISurface = surface.BeginDraw(None, &mut offset)?;
            let drawn = (|| {
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
                let bitmap = self.dc.CreateBitmapFromDxgiSurface(&dxgi, Some(&props))?;
                self.dc.SetTarget(&bitmap);
                self.dc.BeginDraw();
                // The surface can live inside a bigger atlas: draw only in
                // our part of it, at `offset`.
                let (ox, oy) = (offset.x as f32, offset.y as f32);
                self.dc.SetTransform(&Matrix3x2::translation(ox, oy));
                self.dc.PushAxisAlignedClip(
                    &D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: width as f32,
                        bottom: height as f32,
                    },
                    D2D1_ANTIALIAS_MODE_ALIASED,
                );
                self.dc.Clear(Some(&D2D1_COLOR_F::default()));
                self.dc
                    .SetTransform(&Matrix3x2::translation(ox + body.x, oy + body.y));
                self.dc.FillGeometry(&geo, &self.fill, None::<&ID2D1Brush>);
                self.dc.PopAxisAlignedClip();
                let ended = self.dc.EndDraw(None, None);
                self.dc.SetTarget(None::<&ID2D1Image>);
                ended
            })();
            surface.EndDraw()?;
            drawn?;
            self.dcomp.Commit()
        }
    }
}
